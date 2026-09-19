//! Timing / behaviour check for long-document contradiction chunking.
//! Compares sentence-aware overlapping chunks + dedupe helpers (no full LLM
//! required for the structural before/after). Optional `--llm` runs real analysis.
//!
//! cargo run --example long_doc_chunk_bench --manifest-path src-tauri/Cargo.toml --release

use std::path::PathBuf;
use std::time::Instant;

use faqih_ai_lib::commands::document_analysis::dedupe_contradictions_for_test;
use faqih_ai_lib::document_text::{chunk_text, recommended_chunk_overlap};
use faqih_ai_lib::models::ContradictionItem;

fn legacy_char_chunks(text: &str, max_chars: usize, overlap_chars: usize) -> Vec<String> {
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

fn main() {
    let doc = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../testdata/contradiction/long_cross_boundary.txt")
    });
    let text = std::fs::read_to_string(&doc).expect("read doc");
    let max = 3072usize;
    let old_overlap = 400usize;
    let new_overlap = recommended_chunk_overlap(max);

    let t0 = Instant::now();
    let old = legacy_char_chunks(&text, max, old_overlap);
    let old_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let new_chunks = chunk_text(&text, max, new_overlap);
    let new_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let early = "EARLY_MARKER_PAY_8M";
    let late = "LATE_MARKER_PAY_4M";

    let old_bridge = old
        .iter()
        .any(|c| c.contains(early) && c.contains(late));
    let new_bridge = new_chunks
        .iter()
        .any(|c| c.contains(early) && c.contains(late));

    println!("document_chars={}", text.chars().count());
    println!("BEFORE: chunks={} overlap={old_overlap} bridge_both_markers={old_bridge} chunk_ms={old_ms:.3}", old.len());
    println!("AFTER:  chunks={} overlap={new_overlap} bridge_both_markers={new_bridge} chunk_ms={new_ms:.3}", new_chunks.len());

    // Simulate overlap duplicate findings (same pair from two chunks).
    let dupes = vec![
        ContradictionItem {
            statement_a: "EARLY_MARKER_PAY_8M: Xodimga oyiga 8 000 000 (sakkiz million) so‘m ish haqi to‘lanadi.".into(),
            statement_b: "LATE_MARKER_PAY_4M: Xodimning oylik ish haqi 4 000 000 (to‘rt million) so‘m etib belgilangan.".into(),
            explanation: "chunk 1".into(),
        },
        ContradictionItem {
            statement_a: "LATE_MARKER_PAY_4M: Xodimning oylik ish haqi 4 000 000 (to‘rt million) so‘m etib belgilangan.".into(),
            statement_b: "EARLY_MARKER_PAY_8M: Xodimga oyiga 8 000 000 (sakkiz million) so‘m ish haqi to‘lanadi.".into(),
            explanation: "chunk 2 duplicate".into(),
        },
    ];
    let before_n = dupes.len();
    let after = dedupe_contradictions_for_test(dupes);
    println!("DEDUPE: before={before_n} after={}", after.len());
}
