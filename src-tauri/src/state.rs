//! Managed Tauri application state.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::ai::embeddings::EmbeddingEngine;
use crate::ai::llm::{LlmConfig, LlmEngine};
use crate::chat_db::ChatDb;
use crate::db::vector_store::VectorStore;
use crate::models::AppSettings;
use crate::paths;

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub llm: Arc<LlmEngine>,
    pub embeddings: Option<Arc<EmbeddingEngine>>,
    pub vector_store: Option<Arc<VectorStore>>,
    pub chat_db: Mutex<ChatDb>,
}

impl AppState {
    pub fn initialize() -> Result<Self, String> {
        let mut settings = paths::load_settings();
        crate::commands::model_download::hydrate_settings_from_default_model(&mut settings);

        let llm_config = LlmConfig {
            n_ctx: settings.n_ctx.max(512),
            n_threads: settings
                .n_threads
                .map(|n| i32::try_from(n).unwrap_or(paths::default_thread_count()))
                .unwrap_or_else(paths::default_thread_count),
            max_tokens: settings.max_tokens.max(64),
        };
        let llm = Arc::new(LlmEngine::new(llm_config).map_err(|e| e.to_string())?);

        if let Some(path) = settings.model_path.clone() {
            if std::path::Path::new(&path).is_file() {
                llm.load_model_async(path);
            }
        }

        let embeddings = match EmbeddingEngine::load(
            paths::embedding_onnx_path(),
            paths::tokenizer_path(),
            256,
        ) {
            Ok(engine) => Some(Arc::new(engine)),
            Err(err) => {
                tracing::warn!(error = %err, "Embeddings unavailable at startup");
                None
            }
        };

        let vector_store = match VectorStore::open_readonly(paths::lexuz_db_path()) {
            Ok(store) => Some(Arc::new(store)),
            Err(err) => {
                tracing::warn!(error = %err, "LanceDB unavailable at startup");
                None
            }
        };

        let chat_db = ChatDb::open(paths::chat_db_path())?;

        Ok(Self {
            settings: Mutex::new(settings),
            llm,
            embeddings,
            vector_store,
            chat_db: Mutex::new(chat_db),
        })
    }
}
