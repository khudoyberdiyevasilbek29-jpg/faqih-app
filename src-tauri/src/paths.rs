//! Resolve bundled resource paths and app-data settings.
//!
//! Bundled assets (tokenizer, ONNX, lexuz.db) live under Tauri `$RESOURCE` in
//! packaged builds. GGUF is never bundled — path comes from settings / picker.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::models::AppSettings;

static RESOURCE_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Set the directory that contains `models/` and `lexuz.db/` (call from Tauri setup).
pub fn set_resource_root(path: PathBuf) {
    if let Ok(mut guard) = RESOURCE_ROOT.lock() {
        *guard = Some(path);
    }
}

/// Prefer the runtime-configured root; fall back to compile-time `resources/` for tests / early boot.
pub fn resource_root() -> PathBuf {
    if let Ok(guard) = RESOURCE_ROOT.lock() {
        if let Some(path) = guard.as_ref() {
            return path.clone();
        }
    }
    manifest_resources_dir()
}

pub fn manifest_resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources")
}

/// Pick the first candidate that looks like a real resource tree.
pub fn configure_resource_root_from_candidates(candidates: impl IntoIterator<Item = PathBuf>) {
    for candidate in candidates {
        let tokenizer = candidate.join("models").join("tokenizer.json");
        let lexuz = candidate.join("lexuz.db");
        if tokenizer.exists() || lexuz.exists() {
            tracing::info!(path = %candidate.display(), "Using resource root");
            set_resource_root(candidate);
            return;
        }
    }
    let fallback = manifest_resources_dir();
    tracing::warn!(
        path = %fallback.display(),
        "No packaged resource root found; falling back to CARGO_MANIFEST_DIR/resources"
    );
    set_resource_root(fallback);
}

pub fn models_dir() -> PathBuf {
    resource_root().join("models")
}

pub fn embedding_onnx_path() -> PathBuf {
    models_dir().join("multilingual-e5-small.onnx")
}

pub fn tokenizer_path() -> PathBuf {
    models_dir().join("tokenizer.json")
}

pub fn lexuz_db_path() -> PathBuf {
    resource_root().join("lexuz.db")
}

pub fn default_gguf_path() -> PathBuf {
    crate::commands::model_download::default_model_path()
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.mond.faqihai")
}

pub fn settings_path() -> PathBuf {
    app_data_dir().join("settings.json")
}

pub fn chat_db_path() -> PathBuf {
    app_data_dir().join("chat.sqlite")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let dir = app_data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(), raw).map_err(|e| e.to_string())
}

pub fn default_thread_count() -> i32 {
    let n = num_cpus::get();
    i32::try_from(n.saturating_sub(1).max(1)).unwrap_or(1)
}

pub fn resolve_resource(preferred: impl AsRef<Path>) -> PathBuf {
    let preferred = preferred.as_ref();
    if preferred.exists() {
        return preferred.to_path_buf();
    }
    preferred.to_path_buf()
}
