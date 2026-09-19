use std::thread;
use std::time::Duration;

use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::State;

use crate::ai::llm::LlmConfig;
use crate::engine_progress::EngineProgress;
use crate::models::{AppSettings, ModelStatus, SystemStats};
use crate::paths;
use crate::state::AppState;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().clone())
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    let mut settings = settings;
    settings.n_ctx = settings.n_ctx.max(512);
    settings.max_tokens = settings.max_tokens.max(64);
    paths::save_settings(&settings)?;

    let llm_config = LlmConfig {
        n_ctx: settings.n_ctx,
        n_threads: settings
            .n_threads
            .map(|n| i32::try_from(n).unwrap_or_else(|_| paths::default_thread_count()))
            .unwrap_or_else(paths::default_thread_count),
        max_tokens: settings.max_tokens,
    };
    state.llm.update_config(llm_config);
    *state.settings.lock() = settings;
    Ok(())
}

#[tauri::command]
pub fn get_engine_progress(state: State<'_, AppState>) -> Result<EngineProgress, String> {
    Ok(state.engine_progress.lock().clone())
}

#[tauri::command]
pub fn get_model_status(state: State<'_, AppState>) -> Result<ModelStatus, String> {
    let settings = state.settings.lock().clone();
    let loaded = state.llm.is_loaded();
    let loading = state.llm.is_loading();
    let waking = state.llm.is_waking();
    let path = state
        .llm
        .model_path()
        .map(|p| p.display().to_string())
        .or(settings.model_path.clone());
    let name = path.as_ref().map(|p| {
        std::path::Path::new(p)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(p)
            .to_string()
    });
    let progress = state.engine_progress.lock().clone();
    let error = if loaded {
        None
    } else if let Some(err) = progress.error.clone() {
        Some(err)
    } else {
        state.llm.last_error().or_else(|| {
            if path.is_none() {
                Some("AI modeli hali yuklanmagan".into())
            } else if loading || waking || !progress.ready {
                None
            } else {
                // Path configured but weights idle-unloaded — not an error.
                None
            }
        })
    };

    Ok(ModelStatus {
        loaded,
        loading,
        waking,
        path,
        name,
        error,
        template_source: state.llm.template_source(),
    })
}

#[tauri::command]
pub fn set_model_path(state: State<'_, AppState>, path: String) -> Result<ModelStatus, String> {
    let path_buf = std::path::PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Model file not found: {path}"));
    }

    {
        let mut settings = state.settings.lock();
        settings.model_path = Some(path.clone());
        paths::save_settings(&settings)?;
    }

    state.llm.load_model_async(path_buf);
    get_model_status(state)
}

/// Snapshot host RAM/CPU via `sysinfo` (local diagnostics only — no network).
#[tauri::command]
pub fn get_system_stats() -> Result<SystemStats, String> {
    let mut sys = System::new();
    sys.refresh_memory();
    // First CPU sample is often zero; refresh twice with a short gap.
    sys.refresh_cpu_usage();
    thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_usage();

    let pid = Pid::from_u32(std::process::id());
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

    let process_memory_bytes = sys
        .process(pid)
        .map(|p| p.memory())
        .unwrap_or(0);

    Ok(SystemStats {
        total_memory_bytes: sys.total_memory(),
        used_memory_bytes: sys.used_memory(),
        available_memory_bytes: sys.available_memory(),
        process_memory_bytes,
        cpu_usage_percent: sys.global_cpu_usage(),
        cpu_count: sys.cpus().len().max(1),
    })
}
