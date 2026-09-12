//! Retrieval-augmented generation — offline only.

use crate::ai::embeddings::EmbeddingEngine;
use crate::db::vector_store::{RetrievedChunk, VectorStore};
use crate::models::LanguagePreference;

pub struct RagPipeline<'a> {
    embeddings: &'a EmbeddingEngine,
    store: &'a VectorStore,
}

impl<'a> RagPipeline<'a> {
    pub fn new(embeddings: &'a EmbeddingEngine, store: &'a VectorStore) -> Self {
        Self { embeddings, store }
    }

    pub fn retrieve(&self, query: &str, top_k: usize) -> Result<Vec<RetrievedChunk>, String> {
        let vector = self
            .embeddings
            .embed_query(query)
            .map_err(|e| e.to_string())?;
        self.store
            .search(&vector, top_k)
            .map_err(|e| e.to_string())
    }

    pub fn build_system_prompt(
        &self,
        language: LanguagePreference,
        question: &str,
        contexts: &[RetrievedChunk],
    ) -> String {
        let language_instruction = language_instruction(language, question);
        let context_block = if contexts.is_empty() {
            "(No relevant legislation was retrieved for this question.)".to_string()
        } else {
            contexts
                .iter()
                .map(|c| {
                    format!(
                        "[{law} — Art. {art}]\nChapter: {chapter}\nSection: {section}\n{text}",
                        law = c.law_name,
                        art = c.article_number,
                        chapter = c.chapter,
                        section = c.section_title,
                        text = c.text
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n---\n\n")
        };

        format!(
            r#"You are Faqih AI, an offline legal assistant for Uzbekistan legislation (Labor Code / Mehnat kodeksi and Civil Code / Fuqarolik kodeksi only).

STRICT RULES:
1. Answer ONLY using the retrieved legislation below. Do not invent articles, numbers, or rules.
2. Cite law name + article number for every legal claim (e.g. **Mehnat kodeksi, 126-modda**).
3. If the retrieved text does not contain a relevant provision, say clearly that no relevant provision was found — never hallucinate.
4. {language_instruction}
5. Format with Markdown: short paragraphs, bullets for lists, **bold** for key terms and article numbers. Do NOT write one unbroken wall of text for answers longer than 2–3 sentences.

RETRIEVED LEGISLATION:
{context_block}"#
        )
    }

    pub fn assemble_user_prompt(question: &str) -> String {
        question.trim().to_string()
    }
}

fn language_instruction(pref: LanguagePreference, question: &str) -> String {
    match pref {
        LanguagePreference::Uz => {
            "Always respond in Uzbek (Latin script), regardless of the question language.".into()
        }
        LanguagePreference::Ru => {
            "Always respond in Russian, regardless of the question language.".into()
        }
        LanguagePreference::En => {
            "Always respond in English, regardless of the question language.".into()
        }
        LanguagePreference::Auto => {
            let detected = detect_language(question);
            format!(
                "Match the user's language (detected: {detected}). Respond in that language."
            )
        }
    }
}

/// Lightweight offline detector: Cyrillic vs Latin + common stopwords.
pub fn detect_language(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let cyrillic = lower.chars().filter(|c| ('\u{0400}'..='\u{04FF}').contains(c)).count();
    let latin = lower
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .count();

    let ru_hits = ["что", "это", "как", "или", "для", "при", "права", "договор"]
        .iter()
        .filter(|w| lower.contains(*w))
        .count();
    let en_hits = ["the", "and", "what", "which", "contract", "labor", "rights"]
        .iter()
        .filter(|w| lower.split_whitespace().any(|t| t == **w))
        .count();
    let uz_hits = [
        "nima", "qanday", "uchun", "boʻyicha", "boyicha", "modda", "kodeks", "shartnoma", "mehnat",
    ]
    .iter()
    .filter(|w| lower.contains(*w))
    .count();

    if cyrillic > latin || ru_hits >= 2 {
        "ru"
    } else if en_hits >= 2 && en_hits > uz_hits {
        "en"
    } else {
        "uz"
    }
}
