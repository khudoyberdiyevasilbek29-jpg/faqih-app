//! Cold-start / wake progress for LLM, embeddings, and LanceDB.
//!
//! Events are emitted via Tauri AND stored on AppState so the frontend can
//! catch up if it mounts after early stages already finished.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const ENGINE_PROGRESS_EVENT: &str = "engine-progress";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineStage {
    CheckingFiles,
    LoadingLlm,
    LoadingEmbeddings,
    ConnectingDb,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineProgress {
    pub stage: EngineStage,
    /// Machine-readable stage id (mirrors `stage` for TS unions).
    pub stage_id: String,
    /// Optional overall percent 0–100; `None` = indeterminate for this stage.
    pub percent: Option<u8>,
    /// Engines are ready for chat (LLM loaded + embeddings + LanceDB).
    pub ready: bool,
    /// Human-readable Uzbek error when `stage == Error`.
    pub error: Option<String>,
}

impl EngineProgress {
    pub fn stage(stage: EngineStage, percent: Option<u8>) -> Self {
        Self {
            stage_id: stage_id(stage).into(),
            stage,
            percent,
            ready: matches!(stage, EngineStage::Ready),
            error: None,
        }
    }

    pub fn ready() -> Self {
        Self {
            stage: EngineStage::Ready,
            stage_id: stage_id(EngineStage::Ready).into(),
            percent: Some(100),
            ready: true,
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            stage: EngineStage::Error,
            stage_id: stage_id(EngineStage::Error).into(),
            percent: None,
            ready: false,
            error: Some(message.into()),
        }
    }
}

pub fn stage_id(stage: EngineStage) -> &'static str {
    match stage {
        EngineStage::CheckingFiles => "checking_files",
        EngineStage::LoadingLlm => "loading_llm",
        EngineStage::LoadingEmbeddings => "loading_embeddings",
        EngineStage::ConnectingDb => "connecting_db",
        EngineStage::Ready => "ready",
        EngineStage::Error => "error",
    }
}

pub fn publish(app: &AppHandle, progress: EngineProgress) {
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        *state.engine_progress.lock() = progress.clone();
    }
    let _ = app.emit(ENGINE_PROGRESS_EVENT, progress);
}
