//! Managed Tauri application state.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::{AppHandle, Manager};

use crate::ai::embeddings::EmbeddingEngine;
use crate::ai::llm::{LlmConfig, LlmEngine};
use crate::chat_db::ChatDb;
use crate::db::vector_store::VectorStore;
use crate::engine_progress::{self, EngineProgress, EngineStage};
use crate::models::AppSettings;
use crate::paths;

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub llm: Arc<LlmEngine>,
    pub embeddings: Mutex<Option<Arc<EmbeddingEngine>>>,
    pub vector_store: Mutex<Option<Arc<VectorStore>>>,
    pub chat_db: Mutex<ChatDb>,
    pub engine_progress: Mutex<EngineProgress>,
    /// Set true after the first cold-start warm-up finishes (ok or fail).
    pub warmup_finished: AtomicBool,
}

impl AppState {
    /// Fast bootstrap so the window can open and listen for progress events.
    /// Heavy model/DB work runs in [`spawn_engine_warmup`].
    pub fn bootstrap() -> Result<Self, String> {
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
        let chat_db = ChatDb::open(paths::chat_db_path())?;

        Ok(Self {
            settings: Mutex::new(settings),
            llm,
            embeddings: Mutex::new(None),
            vector_store: Mutex::new(None),
            chat_db: Mutex::new(chat_db),
            engine_progress: Mutex::new(EngineProgress::stage(
                EngineStage::CheckingFiles,
                Some(5),
            )),
            warmup_finished: AtomicBool::new(false),
        })
    }
}

/// Run staged engine warm-up on a background thread after `app.manage(state)`.
pub fn spawn_engine_warmup(app: AppHandle) {
    thread::Builder::new()
        .name("faqih-engine-warmup".into())
        .spawn(move || {
            if let Err(err) = warm_up_engines(&app) {
                tracing::error!(error = %err, "Engine warm-up failed");
                if let Some(state) = app.try_state::<AppState>() {
                    state.warmup_finished.store(true, Ordering::SeqCst);
                }
                engine_progress::publish(&app, EngineProgress::error(err));
            }
        })
        .ok();
}

fn warm_up_engines(app: &AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<AppState>() else {
        return Err(
            "Ilova holati tayyor emas. Dasturini qayta ishga tushiring.".into(),
        );
    };

    let finish_ok = || {
        state.warmup_finished.store(true, Ordering::SeqCst);
    };
    let finish_err = |err: String| -> Result<(), String> {
        state.warmup_finished.store(true, Ordering::SeqCst);
        Err(err)
    };

    engine_progress::publish(
        app,
        EngineProgress::stage(EngineStage::CheckingFiles, Some(8)),
    );

    let model_path = state.settings.lock().model_path.clone();
    let gguf_ok = model_path
        .as_ref()
        .map(|p| std::path::Path::new(p).is_file())
        .unwrap_or(false);
    let onnx_path = paths::embedding_onnx_path();
    let tokenizer_path = paths::tokenizer_path();
    let lexuz_path = paths::lexuz_db_path();

    if !onnx_path.is_file() {
        return finish_err(format!(
            "Embedding modeli topilmadi ({}). Ilovani to‘liq o‘rnating yoki resources/models papkasini tekshiring.",
            onnx_path.display()
        ));
    }
    if !tokenizer_path.is_file() {
        return finish_err(format!(
            "Tokenizer fayli topilmadi ({}). Ilovani to‘liq o‘rnating yoki resources/models papkasini tekshiring.",
            tokenizer_path.display()
        ));
    }
    if !lexuz_path.exists() {
        return finish_err(format!(
            "Huquqiy baza topilmadi ({}). Ilovani to‘liq o‘rnating yoki lexuz.db ni tiklang.",
            lexuz_path.display()
        ));
    }

    // Start LLM early when present so it overlaps with embeddings/DB load.
    if gguf_ok {
        engine_progress::publish(
            app,
            EngineProgress::stage(EngineStage::LoadingLlm, Some(20)),
        );
        if let Some(path) = model_path.clone() {
            state.llm.set_app_handle(app.clone());
            state.llm.load_model_async(path);
        }
    } else {
        tracing::info!("GGUF not configured yet; loading embeddings/DB first");
    }

    engine_progress::publish(
        app,
        EngineProgress::stage(EngineStage::LoadingEmbeddings, Some(45)),
    );
    match EmbeddingEngine::load(&onnx_path, &tokenizer_path, 256) {
        Ok(engine) => {
            *state.embeddings.lock() = Some(Arc::new(engine));
        }
        Err(err) => {
            tracing::warn!(error = %err, "Embeddings unavailable at startup");
            return finish_err(format!(
                "Embedding modelini yuklab bo‘lmadi: {err}. ONNX va tokenizer fayllarini tekshirib, dasturni qayta oching."
            ));
        }
    }

    engine_progress::publish(
        app,
        EngineProgress::stage(EngineStage::ConnectingDb, Some(75)),
    );
    match VectorStore::open_readonly(&lexuz_path) {
        Ok(store) => {
            *state.vector_store.lock() = Some(Arc::new(store));
        }
        Err(err) => {
            tracing::warn!(error = %err, "LanceDB unavailable at startup");
            return finish_err(format!(
                "Huquqiy bazaga ulanilmadi: {err}. lexuz.db faylini tekshirib, dasturni qayta oching."
            ));
        }
    }

    if !gguf_ok {
        // First-run / ModelSetup: resources ready; LLM comes after download.
        finish_ok();
        engine_progress::publish(
            app,
            EngineProgress::stage(EngineStage::LoadingLlm, Some(90)),
        );
        tracing::info!("Embeddings + LanceDB ready; waiting for GGUF download");
        return Ok(());
    }

    engine_progress::publish(
        app,
        EngineProgress::stage(EngineStage::LoadingLlm, Some(88)),
    );
    if let Err(err) = wait_for_llm_ready(&state.llm) {
        return finish_err(err);
    }

    finish_ok();
    engine_progress::publish(app, EngineProgress::ready());
    tracing::info!("Engine warm-up complete");
    Ok(())
}

fn wait_for_llm_ready(llm: &LlmEngine) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        if llm.is_loaded() {
            return Ok(());
        }
        if !llm.is_loading() {
            return Err(llm.last_error().unwrap_or_else(|| {
                "Til modelini yuklab bo‘lmadi. Model faylini qayta yuklab oling yoki Sozlamalardan boshqa GGUF tanlang."
                    .into()
            }));
        }
        thread::sleep(Duration::from_millis(150));
    }
    Err(
        "Til modelini yuklash vaqti tugadi. Kompyuter xotirasini bo‘shating va dasturni qayta oching."
            .into(),
    )
}
