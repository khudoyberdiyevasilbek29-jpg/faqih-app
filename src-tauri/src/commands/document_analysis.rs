use std::path::PathBuf;

use serde_json::Value;
use tauri::State;

use crate::document_text::{chunk_text, extract_document_text};
use crate::models::{ContradictionItem, ContradictionReport};
use crate::state::AppState;

#[tauri::command]
pub fn analyze_document(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<ContradictionReport, String> {
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
    let chunks = chunk_text(&text, max_chunk_chars, 400);
    let was_chunked = chunks.len() > 1;

    let mut all_items: Vec<ContradictionItem> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();

    for (idx, chunk) in chunks.iter().enumerate() {
        let system = contradiction_system_prompt(was_chunked, chunks.len(), idx);
        let prompt = state
            .llm
            .format_chat(&system, chunk)
            .map_err(|e| e.to_string())?;
        let (raw, _stop) = state
            .llm
            .generate(&prompt, Some(settings.max_tokens))
            .map_err(|e| e.to_string())?;

        let parsed = parse_contradiction_json(&raw).unwrap_or_else(|| ContradictionReport {
            summary: raw.chars().take(500).collect(),
            contradictions: vec![],
            document_length_chars,
            was_chunked,
        });
        summaries.push(parsed.summary);
        all_items.extend(parsed.contradictions);
    }

    let mut summary = if summaries.len() == 1 {
        summaries.remove(0)
    } else {
        format!(
            "Analyzed {} chunk(s). {}",
            chunks.len(),
            summaries.join(" ")
        )
    };
    if was_chunked {
        summary.push_str(
            " Note: the document exceeded the model context window and was analyzed in overlapping chunks; contradictions that span distant chunks may be missed.",
        );
    }

    Ok(ContradictionReport {
        summary,
        contradictions: all_items,
        document_length_chars,
        was_chunked,
    })
}

fn contradiction_system_prompt(was_chunked: bool, total_chunks: usize, chunk_idx: usize) -> String {
    let chunk_note = if was_chunked {
        format!(
            "This is chunk {} of {}. Focus on contradictions visible within this chunk (and overlapping edges). Do not invent content from outside the provided text.",
            chunk_idx + 1,
            total_chunks
        )
    } else {
        "The full document is provided below.".into()
    };

    format!(
        r#"You are Faqih AI performing INTERNAL document consistency analysis.
This task is NOT legal research against a corpus — do not cite Labor/Civil codes unless the document itself quotes them.

{chunk_note}

Carefully read the document text and identify sentences/statements that contradict each other.
For each contradiction, quote both statements directly from the text and explain the conflict briefly.
Also provide a short overall summary.

Respond with ONLY valid JSON matching this schema (no markdown fences):
{{
  "summary": "string",
  "contradictions": [
    {{
      "statement_a": "direct quote",
      "statement_b": "direct quote",
      "explanation": "why these conflict"
    }}
  ]
}}

If no contradictions are found, return an empty contradictions array and say so in summary.
Use Markdown sparingly inside string values only if needed; prefer plain concise prose."#
    )
}

fn parse_contradiction_json(raw: &str) -> Option<ContradictionReport> {
    let trimmed = raw.trim();
    let json_str = extract_json_object(trimmed)?;
    let value: Value = serde_json::from_str(json_str).ok()?;
    let summary = value
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut contradictions = Vec::new();
    if let Some(arr) = value.get("contradictions").and_then(|v| v.as_array()) {
        for item in arr {
            let statement_a = item
                .get("statement_a")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let statement_b = item
                .get("statement_b")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let explanation = item
                .get("explanation")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if !statement_a.is_empty() || !statement_b.is_empty() {
                contradictions.push(ContradictionItem {
                    statement_a,
                    statement_b,
                    explanation,
                });
            }
        }
    }
    Some(ContradictionReport {
        summary,
        contradictions,
        document_length_chars: 0,
        was_chunked: false,
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
