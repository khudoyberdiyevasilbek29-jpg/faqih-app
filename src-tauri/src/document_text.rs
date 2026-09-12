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
        "txt" | "md" => {
            std::fs::read_to_string(file_path).map_err(|e| e.to_string())
        }
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

/// Chunk long documents with overlap for context-limited models.
pub fn chunk_text(text: &str, max_chars: usize, overlap_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return vec![text.to_string()];
    }
    let overlap = overlap_chars.min(max_chars / 4);
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
