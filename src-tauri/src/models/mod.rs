use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelloWorldResponse {
    pub message: String,
    pub app_name: String,
    pub version: String,
    pub offline: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LanguagePreference {
    #[default]
    Auto,
    Uz,
    Ru,
    En,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub model_path: Option<String>,
    pub n_ctx: u32,
    pub n_threads: Option<u32>,
    pub language: LanguagePreference,
    pub max_tokens: u32,
    #[serde(default)]
    pub has_seen_onboarding: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            model_path: None,
            n_ctx: 4096,
            n_threads: None,
            language: LanguagePreference::Auto,
            max_tokens: 1024,
            has_seen_onboarding: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub loaded: bool,
    pub loading: bool,
    pub waking: bool,
    pub path: Option<String>,
    pub name: Option<String>,
    pub error: Option<String>,
    pub template_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub total_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub process_memory_bytes: u64,
    pub cpu_usage_percent: f32,
    pub cpu_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalReference {
    pub id: String,
    pub code: String,
    pub article: String,
    pub title: String,
    pub excerpt: String,
    pub score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDto {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub references: Vec<LegalReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSessionDto {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatResponse {
    pub session_id: String,
    pub user_message: ChatMessageDto,
    pub assistant_message: ChatMessageDto,
    pub stop_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContradictionItem {
    pub statement_a: String,
    pub statement_b: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContradictionReport {
    pub summary: String,
    pub contradictions: Vec<ContradictionItem>,
    pub document_length_chars: usize,
    pub was_chunked: bool,
}
