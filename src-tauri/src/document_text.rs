use std::path::Path;

use regex::Regex;
use zip::ZipArchive;

pub fn extract_document_text(file_path: &Path) -> Result<String, String> {
    if !file_path.exists() {
        return Err(format!("File not found: {}", file_path.display()));
    }
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "pdf" => extract_pdf(file_path),
        "docx" => extract_docx(file_path),
        "txt" | "md" => std::fs::read_to_string(file_path).map_err(|e| e.to_string()),
        other => Err(format!(
            "Unsupported file type '.{other}'. Supported: pdf, docx, txt, md"
        )),
    }
}

fn extract_pdf(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    pdf_extract::extract_text_from_mem(&bytes).map_err(|e| e.to_string())
}

fn extract_docx(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("Invalid DOCX (missing word/document.xml): {e}"))?;
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut document, &mut xml).map_err(|e| e.to_string())?;

    // Prefer text inside <w:t>...</w:t>, then strip residual tags.
    let re = Regex::new(r"<w:t[^>]*>(.*?)</w:t>").map_err(|e| e.to_string())?;
    let mut parts = Vec::new();
    for cap in re.captures_iter(&xml) {
        if let Some(m) = cap.get(1) {
            parts.push(decode_xml_entities(m.as_str()));
        }
    }
    let text = if parts.is_empty() {
        let stripped = Regex::new(r"<[^>]+>")
            .map_err(|e| e.to_string())?
            .replace_all(&xml, " ");
        decode_xml_entities(&stripped)
    } else {
        parts.join(" ")
    };
    Ok(collapse_ws(&text))
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Recommended overlap for long-document analysis (~1/3 of chunk).
pub fn recommended_chunk_overlap(max_chars: usize) -> usize {
    (max_chars / 3).clamp(600, 2500)
}

/// Chunk long documents with sentence/paragraph-aware boundaries and overlap.
///
/// Prefers breaking on paragraph / sentence / numbered-clause boundaries rather
/// than mid-sentence character cuts. `overlap_chars` is honored up to ~half the
/// chunk size (so progress is still guaranteed).
pub fn chunk_text(text: &str, max_chars: usize, overlap_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(64);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let total = trimmed.chars().count();
    if total <= max_chars {
        return vec![trimmed.to_string()];
    }

    let units = split_into_units(trimmed);
    if units.is_empty() {
        return hard_char_chunks(trimmed, max_chars, overlap_chars);
    }

    // Cap overlap so we always advance through the document.
    let overlap = overlap_chars.min(max_chars / 2).min(max_chars.saturating_sub(64));

    let mut out: Vec<String> = Vec::new();
    let mut start_idx = 0usize;

    while start_idx < units.len() {
        let mut end_idx = start_idx;
        let mut len = 0usize;

        while end_idx < units.len() {
            let unit_len = units[end_idx].chars().count();
            let sep = if end_idx > start_idx { 1 } else { 0 };
            let next = len + sep + unit_len;

            if next > max_chars && end_idx > start_idx {
                break;
            }

            // Oversized single unit: hard-split it.
            if unit_len > max_chars && end_idx == start_idx {
                let hard = hard_char_chunks(&units[end_idx], max_chars, overlap);
                for (hi, piece) in hard.into_iter().enumerate() {
                    if hi == 0 && !out.is_empty() {
                        // Merge first hard piece into previous only if small — keep separate.
                    }
                    out.push(piece);
                }
                start_idx = end_idx + 1;
                end_idx = start_idx;
                break;
            }

            len = next;
            end_idx += 1;
            if len >= max_chars {
                break;
            }
        }

        if end_idx == start_idx {
            // Safety: force one unit forward.
            out.push(units[start_idx].clone());
            start_idx += 1;
            continue;
        }

        if end_idx > start_idx {
            out.push(units[start_idx..end_idx].join(" "));
        }

        if end_idx >= units.len() {
            break;
        }

        // Walk back from end_idx to cover ~overlap chars for the next window.
        let mut back = 0usize;
        let mut new_start = end_idx;
        while new_start > start_idx && back < overlap {
            new_start -= 1;
            back += units[new_start].chars().count() + 1;
        }
        // Must make forward progress relative to previous start.
        if new_start <= start_idx {
            new_start = start_idx + 1;
        }
        // Prefer not to restart at the exact same end (zero new content).
        if new_start >= end_idx {
            new_start = end_idx;
        }
        start_idx = new_start;
    }

    // Drop accidental empties / exact duplicates from hard-split edge cases.
    let mut cleaned = Vec::new();
    for chunk in out {
        let c = chunk.trim().to_string();
        if c.is_empty() {
            continue;
        }
        if cleaned.last().is_some_and(|prev: &String| prev == &c) {
            continue;
        }
        cleaned.push(c);
    }
    if cleaned.is_empty() {
        hard_char_chunks(trimmed, max_chars, overlap)
    } else {
        cleaned
    }
}

fn hard_char_chunks(text: &str, max_chars: usize, overlap_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return vec![text.to_string()];
    }
    let overlap = overlap_chars.min(max_chars / 2);
    let step = max_chars.saturating_sub(overlap).max(1);
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        out.push(chars[start..end].iter().collect());
        if end >= chars.len() {
            break;
        }
        start += step;
    }
    out
}

/// Split into paragraph → sentence / numbered-clause units.
fn split_into_units(text: &str) -> Vec<String> {
    let mut units = Vec::new();
    // Paragraphs: blank lines, or single newlines between dense legal clauses.
    for para in split_paragraphs(text) {
        for sent in split_sentences(&para) {
            let s = collapse_ws(&sent);
            if !s.is_empty() {
                units.push(s);
            }
        }
    }
    units
}

fn split_paragraphs(text: &str) -> Vec<String> {
    let re = Regex::new(r"\n\s*\n+").expect("paragraph regex");
    let parts: Vec<String> = re
        .split(text)
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() > 1 {
        return parts;
    }
    // Fall back: treat each non-empty line as a soft paragraph for clause lists.
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > 1 {
        lines
    } else {
        vec![text.to_string()]
    }
}

fn split_sentences(para: &str) -> Vec<String> {
    let chars: Vec<char> = para.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        let is_end_punct = matches!(c, '.' | '!' | '?' | '…');
        if is_end_punct {
            let mut j = i + 1;
            // Optional closing quotes after punctuation.
            while j < chars.len() && matches!(chars[j], '"' | '\'' | '”' | '»') {
                j += 1;
            }
            // Require whitespace then a plausible next-sentence start.
            if j < chars.len() && chars[j].is_whitespace() {
                let mut k = j;
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if k < chars.len() {
                    let next = chars[k];
                    let next_ok = next.is_uppercase()
                        || next.is_numeric()
                        || matches!(next, '(' | '[' | '«' | '"');
                    if next_ok {
                        let piece: String = chars[start..j].iter().collect();
                        let piece = piece.trim();
                        if !piece.is_empty() {
                            out.push(piece.to_string());
                        }
                        start = k;
                        i = k;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    let tail: String = chars[start..].iter().collect();
    let tail = tail.trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    if out.is_empty() {
        vec![para.to_string()]
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_single_chunk() {
        let text = "Qisqa hujjat. Bitta jumla.";
        let chunks = chunk_text(text, 2000, 600);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("Qisqa"));
    }

    #[test]
    fn prefers_sentence_boundaries() {
        let mut text = String::new();
        for i in 1..=40 {
            text.push_str(&format!(
                "Band {i}. Tomonlar majburiyatni bajaradi va muddatni kuzatadi. "
            ));
        }
        let max = 500usize;
        let chunks = chunk_text(&text, max, 150);
        assert!(chunks.len() > 1);
        for c in &chunks {
            assert!(
                c.chars().count() <= max + 80,
                "chunk unexpectedly oversized: {}",
                c.chars().count()
            );
            // Should usually end on sentence punctuation when possible.
            let trimmed = c.trim_end();
            assert!(
                trimmed.ends_with('.') || trimmed.chars().count() > max.saturating_sub(50),
                "expected sentence-ish end, got: …{}",
                &trimmed[trimmed.len().saturating_sub(20)..]
            );
        }
    }

    #[test]
    fn overlap_keeps_shared_content() {
        let mut text = String::new();
        for i in 1..=30 {
            text.push_str(&format!("Jumla raqami {i} bu yerda turadi. "));
        }
        let chunks = chunk_text(&text, 400, 200);
        assert!(chunks.len() >= 2);
        // Adjacent chunks should share some content due to overlap.
        let a = &chunks[0];
        let b = &chunks[1];
        let a_words: Vec<&str> = a.split_whitespace().collect();
        let shared = a_words
            .iter()
            .rev()
            .take(12)
            .any(|w| b.contains(w) && w.len() > 3);
        assert!(shared, "expected overlap between adjacent chunks");
    }

    #[test]
    fn recommended_overlap_is_about_one_third() {
        assert_eq!(recommended_chunk_overlap(3072), 1024);
        assert!(recommended_chunk_overlap(2000) >= 600);
    }
}
