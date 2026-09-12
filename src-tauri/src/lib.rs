mod ai;
mod chat_db;
mod commands;
mod db;
mod document_text;
mod models;
mod paths;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use commands::{
    chat::{get_chat_history, list_chat_sessions, send_chat_message},
    document_analysis::analyze_document,
    hello::hello_world,
    model_download::{download_default_model, get_model_setup_status},
    settings::{
        get_model_status, get_settings, get_system_stats, save_settings, set_model_path,
    },
};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "faqih_ai=info,faqih_ai_lib=info".into()),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let mut candidates = Vec::new();
            if let Ok(resource_dir) = app.path().resource_dir() {
                candidates.push(resource_dir.clone());
                candidates.push(resource_dir.join("resources"));
            }
            candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources"));
            paths::configure_resource_root_from_candidates(candidates);

            let app_state = AppState::initialize().map_err(|err| {
                tracing::error!(error = %err, "Failed to initialize AppState");
                Box::<dyn std::error::Error>::from(err)
            })?;

            let llm: Arc<_> = Arc::clone(&app_state.llm);
            llm.spawn_idle_unload_watcher();
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hello_world,
            get_settings,
            save_settings,
            get_model_status,
            set_model_path,
            get_system_stats,
            get_model_setup_status,
            download_default_model,
            send_chat_message,
            get_chat_history,
            list_chat_sessions,
            analyze_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Faqih AI");
}
