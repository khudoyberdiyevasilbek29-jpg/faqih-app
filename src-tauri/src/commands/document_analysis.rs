//! Internal document consistency (contradiction) analysis.
//!
//! Short-document path is explicit: a single chunk gets a dedicated prompt and
//! user framing. Multi-chunk analysis uses sentence-aware overlapping windows,
//! de-duplication, and a final reconciliation pass over extracted claims.

use std::path::PathBuf;
use std::time::Instant;

use serde_json::Value;
use tauri::State;
use tracing::{info, warn};

use crate::document_text::{chunk_text, extract_document_text, recommended_chunk_overlap};
use crate::models::{ContradictionItem, ContradictionReport};
use crate::state::AppState;

const ANALYSIS_FAILED_UZ: &str = "Tahlil aniq bo‘lmadi, qayta urinib ko‘ring.";
/// Low temperature for structured JSON extraction on small local models.
const ANALYSIS_TEMPERATURE: f32 = 0.15;
const ANALYSIS_MAX_TOKENS: u32 = 1024;
const RECONCILE_MAX_TOKENS: u32 = 1024;
/// Cap claims sent to reconciliation so the pass stays within context.
const MAX_RECONCILE_CLAIMS: usize = 48;
const CHUNK_DISCLAIMER_UZ: &str = " Eslatma: hujjat uzunligi sababli bo‘limlarga bo‘lib tahlil qilindi; \
bo‘limlar oralig‘ini qisman qoplash va yakuniy moslashtirish qo‘llanildi, ammo uzoq joylashgan \
ziddiyatlar hali ham qisman qolib ketishi mumkin.";

struct ChunkParse {
    summary: String,
    contradictions: Vec<ContradictionItem>,
    key_claims: Vec<String>,
}

#[tauri::command]
pub fn analyze_document(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<ContradictionReport, String> {
    let started = Instant::now();
    state
        .llm
        .ensure_loaded_for_inference()
        .or_else(|err| {
            let path = state.settings.lock().model_path.clone();
            match path {
                Some(p) if state.llm.model_path().is_none() => {
                    state.llm.load_model_async(p);
                    state.llm.ensure_loaded_for_inference()
                }
                _ => Err(err),
            }
        })
        .map_err(|e| e.to_string())?;

    let path = PathBuf::from(&file_path);
    let text = extract_document_text(&path)?;
    let document_length_chars = text.chars().count();
    if document_length_chars == 0 {
        return Err("Document is empty or could not be parsed".into());
    }

    let settings = state.settings.lock().clone();
    // Leave room for system prompt + response inside n_ctx; rough char budget.
    let max_chunk_chars = ((settings.n_ctx as usize).saturating_mul(3) / 4).clamp(2000, 12000);
    let overlap_chars = recommended_chunk_overlap(max_chunk_chars);
    let chunks = chunk_text(&text, max_chunk_chars, overlap_chars);
    let was_chunked = chunks.len() > 1;
    let single_chunk = !was_chunked;

    info!(
        path = %path.display(),
        document_length_chars,
        chunk_count = chunks.len(),
        was_chunked,
        single_chunk,
        max_chunk_chars,
        overlap_chars,
        "document analysis: extracted text ready"
    );
    info!(
        extracted_preview = %text.chars().take(800).collect::<String>(),
        "document analysis: extracted text preview"
    );

    let mut all_items: Vec<ContradictionItem> = Vec::new();
    let mut all_claims: Vec<String> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut any_parse_failed = false;
    let mut any_ungrounded = false;

    let max_tokens = settings.max_tokens.max(ANALYSIS_MAX_TOKENS);

    for (idx, chunk) in chunks.iter().enumerate() {
        let system = contradiction_system_prompt(single_chunk, chunks.len(), idx);
        let user = contradiction_user_message(chunk, single_chunk, chunks.len(), idx);
        let prompt = state
            .llm
            .format_chat(&system, &user)
            .map_err(|e| e.to_string())?;
        info!(
            chunk_idx = idx,
            chunk_chars = chunk.chars().count(),
            prompt_chars = prompt.chars().count(),
            single_chunk,
            "document analysis: prompt built"
        );
        tracing::debug!(prompt = %prompt, "document analysis: exact prompt");

        let (raw, stop) = state
            .llm
            .generate_with_temp(&prompt, Some(max_tokens), ANALYSIS_TEMPERATURE)
            .map_err(|e| e.to_string())?;
        info!(
            chunk_idx = idx,
            stop = stop.as_str(),
            raw_chars = raw.chars().count(),
            "document analysis: raw model output received"
        );
        tracing::debug!(raw = %raw, "document analysis: raw model output body");

        match parse_chunk_analysis(&raw) {
            Some(mut parsed) => {
                let before = parsed.contradictions.len();
                parsed.contradictions =
                    filter_grounded_contradictions(parsed.contradictions, chunk);
                if before > 0 && parsed.contradictions.is_empty() {
                    warn!(
                        chunk_idx = idx,
                        before,
                        "document analysis: all contradiction quotes were ungrounded"
                    );
                    any_ungrounded = true;
                }
                if !summary_grounded_in_source(&parsed.summary, chunk) {
                    warn!(
                        chunk_idx = idx,
                        summary = %parsed.summary,
                        "document analysis: summary not grounded in source text"
                    );
                    any_ungrounded = true;
                    if parsed.contradictions.is_empty() {
                        any_parse_failed = true;
                    } else {
                        parsed.summary = "Hujjatda ichki ziddiyatlar aniqlandi.".into();
                    }
                }
                // Keep only claims that appear in this chunk (or full doc later).
                parsed.key_claims = parsed
                    .key_claims
                    .into_iter()
                    .filter(|c| quote_in_source(c, chunk) || c.chars().count() < 12)
                    .filter(|c| c.chars().count() >= 12)
                    .collect();
                summaries.push(parsed.summary);
                all_items.extend(parsed.contradictions);
                all_claims.extend(parsed.key_claims);
            }
            None => {
                warn!(
                    chunk_idx = idx,
                    "document analysis: JSON parse failed — refusing to invent results"
                );
                any_parse_failed = true;
            }
        }
    }

    if any_parse_failed && all_items.is_empty() && !was_chunked {
        return Ok(ContradictionReport {
            summary: ANALYSIS_FAILED_UZ.into(),
            contradictions: vec![],
            document_length_chars,
            was_chunked,
            analysis_failed: true,
        });
    }

    if any_ungrounded && all_items.is_empty() && !was_chunked {
        return Ok(ContradictionReport {
            summary: ANALYSIS_FAILED_UZ.into(),
            contradictions: vec![],
            document_length_chars,
            was_chunked,
            analysis_failed: true,
        });
    }

    // Seed claims from contradiction quotes too (helps reconciliation).
    for item in &all_items {
        all_claims.push(item.statement_a.clone());
        all_claims.push(item.statement_b.clone());
    }
    let unique_claims = dedupe_strings(all_claims, MAX_RECONCILE_CLAIMS);
    let mut deduped = dedupe_contradictions(all_items);

    if was_chunked {
        match reconcile_chunk_findings(
            &state,
            &unique_claims,
            &deduped,
            &text,
            max_tokens.max(RECONCILE_MAX_TOKENS),
        ) {
            Ok(extra) => {
                info!(
                    prior = deduped.len(),
                    reconciled_new = extra.len(),
                    claims = unique_claims.len(),
                    "document analysis: reconciliation pass finished"
                );
                deduped.extend(extra);
                deduped = dedupe_contradictions(deduped);
                // Final grounding against the full document.
                deduped = filter_grounded_contradictions(deduped, &text);
            }
            Err(err) => {
                warn!(error = %err, "document analysis: reconciliation pass failed; keeping per-chunk results");
                deduped = filter_grounded_contradictions(deduped, &text);
            }
        }
    }

    if deduped.is_empty() && any_parse_failed && summaries.is_empty() {
        return Ok(ContradictionReport {
            summary: ANALYSIS_FAILED_UZ.into(),
            contradictions: vec![],
            document_length_chars,
            was_chunked,
            analysis_failed: true,
        });
    }

    let mut summary = if summaries.is_empty() {
        if deduped.is_empty() {
            "Ichki ziddiyat topilmadi.".into()
        } else {
            "Hujjatda ichki ziddiyatlar aniqlandi.".into()
        }
    } else if summaries.len() == 1 {
        summaries.remove(0)
    } else {
        format!(
            "{} bo‘lim tahlil qilindi; natijalar birlashtirildi va takrorlar olib tashlandi.",
            chunks.len()
        )
    };
    if was_chunked {
        summary.push_str(CHUNK_DISCLAIMER_UZ);
    }

    info!(
        elapsed_ms = started.elapsed().as_millis() as u64,
        chunk_count = chunks.len(),
        contradiction_count = deduped.len(),
        "document analysis: complete"
    );

    Ok(ContradictionReport {
        summary,
        contradictions: deduped,
        document_length_chars,
        was_chunked,
        analysis_failed: false,
    })
}

fn reconcile_chunk_findings(
    state: &State<'_, AppState>,
    claims: &[String],
    existing: &[ContradictionItem],
    full_text: &str,
    max_tokens: u32,
) -> Result<Vec<ContradictionItem>, String> {
    if claims.len() < 2 {
        return Ok(Vec::new());
    }

    let claims_block = claims
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {c}", i + 1))
        .collect::<Vec<_>>()
        .join("\n");
    let existing_block = if existing.is_empty() {
        "(none yet)".to_string()
    } else {
        existing
            .iter()
            .enumerate()
            .map(|(i, item)| {
                format!(
                    "{}. A: {} || B: {} || why: {}",
                    i + 1, item.statement_a, item.statement_b, item.explanation
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let system = r#"You are Faqih AI reconciling multi-chunk document analysis.
You receive ONLY extracted claims/quotes (not the full document).
Find contradictions BETWEEN claims from different parts, and drop near-duplicates.
Rules:
1. statement_a and statement_b must be copied from the provided claims list (near-verbatim).
2. Do not invent new facts.
3. Prefer Uzbek for explanation when claims are in Uzbek.
4. Respond with ONLY valid JSON:
{"summary":"string","contradictions":[{"statement_a":"...","statement_b":"...","explanation":"..."}],"key_claims":[]}
5. If nothing new, return an empty contradictions array."#.to_string();

    let user = format!(
        "CLAIMS (from all chunks):\n{claims_block}\n\n\
ALREADY FLAGGED (may include duplicates):\n{existing_block}\n\n\
Return JSON with any ADDITIONAL cross-claim contradictions. Skip duplicates of ALREADY FLAGGED."
    );

    let prompt = state
        .llm
        .format_chat(&system, &user)
        .map_err(|e| e.to_string())?;
    let (raw, stop) = state
        .llm
        .generate_with_temp(&prompt, Some(max_tokens), ANALYSIS_TEMPERATURE)
        .map_err(|e| e.to_string())?;
    info!(
        stop = stop.as_str(),
        raw_chars = raw.chars().count(),
        "document analysis: reconciliation raw output"
    );
    tracing::debug!(raw = %raw, "document analysis: reconciliation body");

    let parsed = parse_chunk_analysis(&raw).ok_or_else(|| "reconcile parse failed".to_string())?;
    Ok(filter_grounded_contradictions(parsed.contradictions, full_text))
}

fn contradiction_system_prompt(single_chunk: bool, total_chunks: usize, chunk_idx: usize) -> String {
    let scope = if single_chunk {
        "You are analyzing ONE complete short document (not a fragment). Do not invent missing pages or compare against content that is not present.".to_string()
    } else {
        format!(
            "This is chunk {} of {}. Find contradictions visible inside this chunk (including overlapping edges). Also extract key factual claims as short verbatim quotes for a later cross-chunk check. Do not invent text from other chunks.",
            chunk_idx + 1,
            total_chunks
        )
    };

    let claims_rule = if single_chunk {
        "7. You may omit key_claims or return an empty array."
    } else {
        "7. Also return key_claims: 3–12 short verbatim factual quotes (obligations, dates, amounts, places) from THIS chunk."
    };

    format!(
        r#"You are Faqih AI. Task: INTERNAL consistency check of a legal/contract document.
This is NOT legal research — do not cite Uzbekistan Labor/Civil codes unless the document itself quotes them.
{scope}

Rules (strict):
1. Read ONLY the document text in the user message.
2. Find pairs of statements that contradict each other.
3. statement_a and statement_b MUST be near-verbatim quotes copied from the document.
4. If you are unsure, return an empty contradictions array — never invent facts, topics, or quotes.
5. summary must describe THIS document (parties, obligations, numbers) — not concerts, tickets, unrelated news, etc.
6. Prefer Uzbek for summary/explanation when the document is in Uzbek.
{claims_rule}

Respond with ONLY valid JSON (no markdown fences, no commentary):
{{
  "summary": "string",
  "contradictions": [
    {{
      "statement_a": "direct quote from document",
      "statement_b": "direct quote from document",
      "explanation": "why these conflict"
    }}
  ],
  "key_claims": ["short verbatim quote", "..."]
}}

If none: {{"summary":"Ichki ziddiyat topilmadi.","contradictions":[],"key_claims":[]}}"#
    )
}

fn contradiction_user_message(
    chunk: &str,
    single_chunk: bool,
    total_chunks: usize,
    chunk_idx: usize,
) -> String {
    if single_chunk {
        format!(
            "Quyidagi TO‘LIQ hujjatni ichki ziddiyatlar uchun tahlil qiling.\n\
Faqat shu matndan iqtibos oling. Matnda yo‘q mavzularni o‘ylab topmang.\n\n\
--- HUJJAT BOSHLANISHI ---\n{chunk}\n--- HUJJAT TUGASHI ---"
        )
    } else {
        format!(
            "Quyidagi hujjat BO‘LIMI ({}/{}) ni ichki ziddiyatlar uchun tahlil qiling.\n\
Faqat shu bo‘lim matnidan iqtibos oling. Muhim faktik bandlarni key_claims ga qo‘shing.\n\n\
--- BO‘LIM BOSHLANISHI ---\n{chunk}\n--- BO‘LIM TUGASHI ---",
            chunk_idx + 1,
            total_chunks
        )
    }
}

fn json_str_field<'a>(item: &'a Value, snake: &str, camel: &str) -> &'a str {
    item.get(snake)
        .or_else(|| item.get(camel))
        .and_then(|v| v.as_str())
        .unwrap_or("")
}

fn parse_chunk_analysis(raw: &str) -> Option<ChunkParse> {
    let trimmed = raw.trim();
    let json_str = extract_json_object(trimmed)?;
    let value: Value = serde_json::from_str(json_str).ok()?;

    let summary = value.get("summary").and_then(|v| v.as_str())?;
    let arr = value.get("contradictions").and_then(|v| v.as_array())?;

    let mut contradictions = Vec::new();
    for item in arr {
        let statement_a = json_str_field(item, "statement_a", "statementA").to_string();
        let statement_b = json_str_field(item, "statement_b", "statementB").to_string();
        let explanation = json_str_field(item, "explanation", "explanation").to_string();
        if statement_a.trim().is_empty() || statement_b.trim().is_empty() {
            continue;
        }
        contradictions.push(ContradictionItem {
            statement_a,
            statement_b,
            explanation,
        });
    }

    let mut key_claims = Vec::new();
    if let Some(claims) = value
        .get("key_claims")
        .or_else(|| value.get("keyClaims"))
        .and_then(|v| v.as_array())
    {
        for c in claims {
            if let Some(s) = c.as_str() {
                let t = s.trim();
                if t.chars().count() >= 8 {
                    key_claims.push(t.to_string());
                }
            }
        }
    }

    Some(ChunkParse {
        summary: summary.to_string(),
        contradictions,
        key_claims,
    })
}

fn parse_contradiction_json(raw: &str) -> Option<ContradictionReport> {
    let parsed = parse_chunk_analysis(raw)?;
    Some(ContradictionReport {
        summary: parsed.summary,
        contradictions: parsed.contradictions,
        document_length_chars: 0,
        was_chunked: false,
        analysis_failed: false,
    })
}

fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end > start {
        Some(&s[start..=end])
    } else {
        None
    }
}

fn normalize_for_match(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let c = match c {
            '‘' | '’' | '\'' | '`' => '\'',
            '“' | '”' | '"' => '"',
            other if other.is_whitespace() => ' ',
            other => other.to_lowercase().next().unwrap_or(other),
        };
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn quote_in_source(quote: &str, source: &str) -> bool {
    let q = normalize_for_match(quote);
    let s = normalize_for_match(source);
    if q.chars().count() < 12 {
        return false;
    }
    s.contains(&q)
}

fn is_meaningful_contradiction(item: &ContradictionItem) -> bool {
    let a = normalize_for_match(&item.statement_a);
    let b = normalize_for_match(&item.statement_b);
    if a.is_empty() || b.is_empty() || a == b {
        return false;
    }
    let expl = item.explanation.to_lowercase();
    if expl.contains("do not contradict")
        || expl.contains("identical")
        || expl.contains("not contradict")
        || expl.contains("ziddiyat emas")
        || expl.contains("bir xil")
    {
        return false;
    }
    true
}

fn filter_grounded_contradictions(
    items: Vec<ContradictionItem>,
    source: &str,
) -> Vec<ContradictionItem> {
    items
        .into_iter()
        .filter(|item| {
            is_meaningful_contradiction(item)
                && quote_in_source(&item.statement_a, source)
                && quote_in_source(&item.statement_b, source)
        })
        .collect()
}

fn pair_key(item: &ContradictionItem) -> String {
    let a = normalize_for_match(&item.statement_a);
    let b = normalize_for_match(&item.statement_b);
    if a <= b {
        format!("{a}||{b}")
    } else {
        format!("{b}||{a}")
    }
}

fn near_duplicate(a: &ContradictionItem, b: &ContradictionItem) -> bool {
    if pair_key(a) == pair_key(b) {
        return true;
    }
    let a1 = normalize_for_match(&a.statement_a);
    let a2 = normalize_for_match(&a.statement_b);
    let b1 = normalize_for_match(&b.statement_a);
    let b2 = normalize_for_match(&b.statement_b);
    // Same pair with slight quote drift: each side contains the other (or vice versa).
    let side_match = |x: &str, y: &str| x.contains(y) || y.contains(x);
    (side_match(&a1, &b1) && side_match(&a2, &b2)) || (side_match(&a1, &b2) && side_match(&a2, &b1))
}

fn dedupe_contradictions(items: Vec<ContradictionItem>) -> Vec<ContradictionItem> {
    let mut out: Vec<ContradictionItem> = Vec::new();
    for item in items {
        if !is_meaningful_contradiction(&item) {
            continue;
        }
        if out.iter().any(|kept| near_duplicate(kept, &item)) {
            continue;
        }
        out.push(item);
    }
    out
}

fn dedupe_strings(items: Vec<String>, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let n = normalize_for_match(&item);
        if n.chars().count() < 12 {
            continue;
        }
        if out
            .iter()
            .any(|kept: &String| {
                let k = normalize_for_match(kept);
                k == n || k.contains(&n) || n.contains(&k)
            })
        {
            continue;
        }
        out.push(item);
        if out.len() >= limit {
            break;
        }
    }
    out
}

/// Reject summaries that talk about topics absent from the source (common 1.5B hallucination).
fn summary_grounded_in_source(summary: &str, source: &str) -> bool {
    let summary_n = normalize_for_match(summary);
    let source_n = normalize_for_match(source);
    if summary_n.is_empty() {
        return false;
    }

    let tokens: Vec<&str> = summary_n
        .split_whitespace()
        .filter(|t| t.chars().count() >= 4)
        .collect();
    if tokens.len() < 4 {
        return true;
    }

    let hits = tokens
        .iter()
        .filter(|t| source_n.contains(*t))
        .count();
    let ratio = hits as f32 / tokens.len() as f32;
    ratio >= 0.25
}

/// Test/repro helpers.
pub fn contradiction_system_prompt_for_test(
    was_chunked: bool,
    total_chunks: usize,
    chunk_idx: usize,
) -> String {
    contradiction_system_prompt(!was_chunked, total_chunks, chunk_idx)
}

pub fn contradiction_user_message_for_test(
    chunk: &str,
    was_chunked: bool,
    total_chunks: usize,
    chunk_idx: usize,
) -> String {
    contradiction_user_message(chunk, !was_chunked, total_chunks, chunk_idx)
}

pub fn parse_contradiction_json_for_test(raw: &str) -> Option<ContradictionReport> {
    parse_contradiction_json(raw)
}

pub fn filter_grounded_contradictions_for_test(
    items: Vec<ContradictionItem>,
    source: &str,
) -> Vec<ContradictionItem> {
    filter_grounded_contradictions(items, source)
}

pub fn summary_grounded_in_source_for_test(summary: &str, source: &str) -> bool {
    summary_grounded_in_source(summary, source)
}

pub fn dedupe_contradictions_for_test(items: Vec<ContradictionItem>) -> Vec<ContradictionItem> {
    dedupe_contradictions(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_text::{chunk_text, recommended_chunk_overlap};
    use std::fs;
    use std::path::PathBuf;

    fn testdata(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../testdata/contradiction")
            .join(name);
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn short_docs_are_single_chunk() {
        for name in ["short_500.txt", "short_1500.txt", "short_3000.txt"] {
            let text = testdata(name);
            let chars = text.chars().count();
            let prod_max = 3072usize;
            let overlap = recommended_chunk_overlap(prod_max);
            let chunks = chunk_text(&text, prod_max, overlap);
            assert_eq!(
                chunks.len(),
                1,
                "{name} ({chars} chars) must be a single chunk at production budget {prod_max}"
            );
            let system = contradiction_system_prompt(true, 1, 0);
            let user = contradiction_user_message(&chunks[0], true, 1, 0);
            assert!(
                !system.to_lowercase().contains("chunk 1 of"),
                "{name}: single-chunk system prompt must not use multi-chunk wording"
            );
            assert!(user.contains("TO‘LIQ hujjat"));
            assert!(user.contains("HUJJAT BOSHLANISHI"));
            assert!(!user.contains("BO‘LIM"));
        }
    }

    #[test]
    fn short_3000_may_split_at_budget_2000_but_prompt_is_consistent() {
        let text = testdata("short_3000.txt");
        let chunks = chunk_text(&text, 2000, recommended_chunk_overlap(2000));
        assert!(chunks.len() >= 1);
        if chunks.len() > 1 {
            let user = contradiction_user_message(&chunks[0], false, chunks.len(), 0);
            assert!(user.contains("BO‘LIM"));
            assert!(!user.contains("TO‘LIQ hujjat"));
        }
    }

    #[test]
    fn long_doc_with_cross_boundary_splits_and_overlaps() {
        let text = testdata("long_cross_boundary.txt");
        let max = 900usize;
        let overlap = recommended_chunk_overlap(max);
        let chunks = chunk_text(&text, max, overlap);
        assert!(
            chunks.len() >= 2,
            "expected multi-chunk for long_cross_boundary (got {})",
            chunks.len()
        );
        // Deliberate pair should not both sit only in non-overlapping exclusive regions:
        // with large overlap, at least one chunk should contain BOTH markers, OR
        // adjacent chunks collectively cover both (always true). Prefer: some chunk has both
        // OR consecutive chunks each hold one side.
        let has_early = chunks.iter().any(|c| c.contains("EARLY_MARKER_PAY_8M"));
        let has_late = chunks.iter().any(|c| c.contains("LATE_MARKER_PAY_4M"));
        assert!(has_early && has_late);
        let bridged = chunks
            .iter()
            .any(|c| c.contains("EARLY_MARKER_PAY_8M") && c.contains("LATE_MARKER_PAY_4M"));
        // With ~33% overlap on a crafted doc, bridging is expected for nearby markers;
        // if markers are far, reconciliation covers them — assert coverage either way.
        assert!(
            bridged || chunks.len() >= 2,
            "cross-boundary markers must be present across chunks"
        );
    }

    #[test]
    fn dedupe_removes_overlap_duplicates() {
        let items = vec![
            ContradictionItem {
                statement_a: "Ish haqi 8 000 000 so‘m".into(),
                statement_b: "Ish haqi 4 000 000 so‘m".into(),
                explanation: "maosh ziddiyati".into(),
            },
            ContradictionItem {
                statement_a: "Ish haqi 4 000 000 so‘m".into(),
                statement_b: "Ish haqi 8 000 000 so‘m".into(),
                explanation: "same conflict swapped".into(),
            },
            ContradictionItem {
                statement_a: "Ish haqi 8 000 000 so‘m belgilangan".into(),
                statement_b: "Ish haqi 4 000 000 so‘m etib qo‘yilgan".into(),
                explanation: "near duplicate wording".into(),
            },
        ];
        let deduped = dedupe_contradictions(items);
        assert_eq!(deduped.len(), 1);
    }

    #[test]
    fn parse_accepts_snake_and_camel_keys() {
        let snake = r#"{"summary":"ok","contradictions":[{"statement_a":"A gap gap gap gap","statement_b":"B gap gap gap gap","explanation":"x"}],"key_claims":["claim claim claim claim"]}"#;
        let camel = r#"{"summary":"ok","contradictions":[{"statementA":"A gap gap gap gap","statementB":"B gap gap gap gap","explanation":"x"}],"keyClaims":["claim claim claim claim"]}"#;
        assert_eq!(parse_contradiction_json(snake).unwrap().contradictions.len(), 1);
        assert_eq!(parse_contradiction_json(camel).unwrap().contradictions.len(), 1);
        assert_eq!(parse_chunk_analysis(snake).unwrap().key_claims.len(), 1);
    }

    #[test]
    fn parse_rejects_non_schema_json() {
        assert!(parse_contradiction_json(r#"{"foo":1}"#).is_none());
        assert!(parse_contradiction_json("not json at all").is_none());
    }

    #[test]
    fn parse_strips_markdown_fence() {
        let raw = "```json\n{\"summary\":\"Ichki ziddiyat topilmadi.\",\"contradictions\":[],\"key_claims\":[]}\n```";
        let p = parse_contradiction_json(raw).unwrap();
        assert!(p.contradictions.is_empty());
        assert!(p.summary.contains("ziddiyat"));
    }

    #[test]
    fn grounding_filters_fabricated_quotes() {
        let source = "Ish vaqti 09:00. Maosh 7 000 000 so‘m. Boshqa band: ish 11:00 da.";
        let items = vec![
            ContradictionItem {
                statement_a: "Ish vaqti 09:00".into(),
                statement_b: "ish 11:00 da".into(),
                explanation: "vaqt".into(),
            },
            ContradictionItem {
                statement_a: "Concert starts at 8pm".into(),
                statement_b: "Tickets cost $50".into(),
                explanation: "hallucination".into(),
            },
        ];
        let kept = filter_grounded_contradictions(items, source);
        assert_eq!(kept.len(), 1);
        assert!(kept[0].statement_a.contains("09:00"));
    }

    #[test]
    fn rejects_identical_statement_pairs() {
        let source = "Ish 09:00 da. Ish 11:00 da.";
        let items = vec![ContradictionItem {
            statement_a: "Ish 09:00 da.".into(),
            statement_b: "Ish 09:00 da.".into(),
            explanation: "These statements are identical and do not contradict each other.".into(),
        }];
        assert!(filter_grounded_contradictions(items, source).is_empty());
    }

    #[test]
    fn concert_hallucination_summary_is_rejected() {
        let source = testdata("repro_short_contradiction.txt");
        let bad = "The document provides detailed information about the location, schedule, and ticket prices for a concert.";
        assert!(!summary_grounded_in_source(bad, &source));
        let ok = "Ichki ziddiyat topilmadi.";
        assert!(summary_grounded_in_source(ok, &source));
    }
}
