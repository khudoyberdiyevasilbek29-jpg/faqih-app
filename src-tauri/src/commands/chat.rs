use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::ai::rag::RagPipeline;
use crate::models::{
    ChatMessageDto, ChatSessionDto, LegalReference, SendChatResponse,
};
use crate::state::AppState;

pub const CHAT_TOKEN_EVENT: &str = "chat-token";
pub const CHAT_STREAM_END_EVENT: &str = "chat-stream-end";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTokenEvent {
    pub token: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatStreamEndEvent {
    pub stop_reason: String,
}

#[tauri::command]
pub fn list_chat_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSessionDto>, String> {
    state.chat_db.lock().list_sessions()
}

#[tauri::command]
pub fn get_chat_history(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ChatMessageDto>, String> {
    state.chat_db.lock().get_history(&session_id)
}

#[tauri::command]
pub fn send_chat_message(
    app: AppHandle,
    state: State<'_, AppState>,
    message: String,
    session_id: Option<String>,
) -> Result<SendChatResponse, String> {
    let message = message.trim().to_string();
    if message.is_empty() {
        return Err("Message must not be empty".into());
    }
    state
        .llm
        .ensure_loaded_for_inference()
        .or_else(|err| {
            // Settings may have a path even if the engine never finished a load.
            let path = state.settings.lock().model_path.clone();
            match path {
                Some(p) if state.llm.model_path().is_none() => {
                    state.llm.load_model_async(p);
                    state.llm.ensure_loaded_for_inference()
                }
                _ => Err(err),
            }
        })
        .map_err(|e| e.to_string())?;

    let embeddings = state
        .embeddings
        .as_ref()
        .ok_or_else(|| "Embedding engine not available (missing ONNX/tokenizer resources)".to_string())?;
    let store = state
        .vector_store
        .as_ref()
        .ok_or_else(|| "Legal vector DB not available (resources/lexuz.db)".to_string())?;

    let settings = state.settings.lock().clone();
    let rag = RagPipeline::new(embeddings.as_ref(), store.as_ref());
    let contexts = rag.retrieve(&message, 5)?;
    let system = rag.build_system_prompt(settings.language, &message, &contexts);
    let user = RagPipeline::assemble_user_prompt(&message);
    let prompt = state
        .llm
        .format_chat(&system, &user)
        .map_err(|e| e.to_string())?;

    let app_for_stream = app.clone();
    let (answer, stop_reason) = state
        .llm
        .generate_stream(&prompt, Some(settings.max_tokens), move |piece| {
            let _ = app_for_stream.emit(
                CHAT_TOKEN_EVENT,
                ChatTokenEvent {
                    token: piece.to_string(),
                },
            );
        })
        .map_err(|e| e.to_string())?;

    let _ = app.emit(
        CHAT_STREAM_END_EVENT,
        ChatStreamEndEvent {
            stop_reason: stop_reason.as_str().to_string(),
        },
    );

    let references: Vec<LegalReference> = contexts
        .iter()
        .map(|c| LegalReference {
            id: c.id.clone(),
            code: c.law_name.clone(),
            article: c.article_number.clone(),
            title: c.section_title.clone(),
            excerpt: c.text.chars().take(400).collect(),
            score: Some(c.score),
        })
        .collect();

    let db = state.chat_db.lock();
    let session_id = db.ensure_session(session_id, &message)?;
    let user_message = db.append_message(&session_id, "user", &message, &[])?;
    let assistant_message =
        db.append_message(&session_id, "assistant", &answer, &references)?;

    Ok(SendChatResponse {
        session_id,
        user_message,
        assistant_message,
        stop_reason: stop_reason.as_str().to_string(),
    })
}
