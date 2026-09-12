use crate::models::HelloWorldResponse;

/// Minimal IPC smoke test — no AI logic.
#[tauri::command]
pub fn hello_world(name: String) -> HelloWorldResponse {
    let trimmed = name.trim();
    let who = if trimmed.is_empty() { "world" } else { trimmed };

    HelloWorldResponse {
        message: format!(
            "Hello, {who}! Faqih AI Rust bridge is online (100% offline)."
        ),
        app_name: "Faqih AI by MOND".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        offline: true,
    }
}
