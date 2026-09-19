//! One-shot reproduction of short-document contradiction analysis.
//! Run: `cargo run --example repro_short_doc --manifest-path src-tauri/Cargo.toml --release -- /abs/path/to.txt`

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use faqih_ai_lib::ai::llm::{LlmConfig, LlmEngine};
use faqih_ai_lib::commands::document_analysis::{
    contradiction_system_prompt_for_test, contradiction_user_message_for_test,
    filter_grounded_contradictions_for_test, parse_contradiction_json_for_test,
    summary_grounded_in_source_for_test,
};
use faqih_ai_lib::document_text::{chunk_text, extract_document_text};

fn main() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("faqih_ai_lib=info")
        .try_init();

    let doc = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../testdata/contradiction/repro_short_contradiction.txt")
        });

    let text = extract_document_text(&doc).expect("extract");
    eprintln!("=== EXTRACTED TEXT ({} chars) ===\n{}\n", text.chars().count(), text);

    let max_chunk = 2000usize;
    let chunks = chunk_text(&text, max_chunk, 400);
    let was_chunked = chunks.len() > 1;
    eprintln!(
        "=== CHUNKS: {} (single_chunk={}) ===",
        chunks.len(),
        !was_chunked
    );

    let model = dirs::data_dir()
        .unwrap()
        .join("com.mond.faqihai/models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf");
    eprintln!("=== MODEL: {} ===", model.display());

    let engine = Arc::new(
        LlmEngine::new(LlmConfig {
            n_ctx: 4096,
            n_threads: 4,
            max_tokens: 1024,
        })
        .expect("backend"),
    );
    engine.load_model_async(&model);
    let deadline = std::time::Instant::now() + Duration::from_secs(180);
    while !engine.is_loaded() {
        if std::time::Instant::now() > deadline {
            panic!("model load timeout: {:?}", engine.last_error());
        }
        if !engine.is_loading() && engine.last_error().is_some() {
            panic!("model load failed: {:?}", engine.last_error());
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    for (idx, chunk) in chunks.iter().enumerate() {
        let system = contradiction_system_prompt_for_test(was_chunked, chunks.len(), idx);
        let user = contradiction_user_message_for_test(chunk, was_chunked, chunks.len(), idx);
        let prompt = engine.format_chat(&system, &user).expect("format");
        eprintln!("=== PROMPT ({} chars) ===\n{}\n", prompt.chars().count(), prompt);

        let (raw, stop) = engine
            .generate_with_temp(&prompt, Some(1024), 0.15)
            .expect("generate");
        eprintln!("=== RAW MODEL OUTPUT (stop={stop:?}) ===\n{raw}\n");

        match parse_contradiction_json_for_test(&raw) {
            Some(report) => {
                let grounded =
                    filter_grounded_contradictions_for_test(report.contradictions.clone(), chunk);
                let summary_ok = summary_grounded_in_source_for_test(&report.summary, chunk);
                eprintln!(
                    "=== PARSED: summary_grounded={summary_ok} items={} grounded_items={} summary={:?} ===",
                    report.contradictions.len(),
                    grounded.len(),
                    report.summary
                );
                for (i, c) in grounded.iter().enumerate() {
                    eprintln!(
                        "  [{i}] A={:?}\n      B={:?}\n      why={:?}",
                        c.statement_a, c.statement_b, c.explanation
                    );
                }
                if !summary_ok && grounded.is_empty() {
                    eprintln!("=== WOULD FAIL HONESTLY: Tahlil aniq bo‘lmadi, qayta urinib ko‘ring. ===");
                } else if report.contradictions.len() > grounded.len() && grounded.is_empty() {
                    eprintln!("=== WOULD FAIL HONESTLY (ungrounded quotes): Tahlil aniq bo‘lmadi, qayta urinib ko‘ring. ===");
                }
            }
            None => eprintln!("=== PARSE FAILED → honest fallback ==="),
        }
    }
}
