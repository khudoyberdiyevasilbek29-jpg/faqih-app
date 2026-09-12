//! llama-cpp-2 wrapper with known-bug preventions applied from day one.

use std::num::{NonZeroU16, NonZeroU32};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use encoding_rs::UTF_8;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use tracing::{info, warn};

use crate::paths;

const PENALTY_REPEAT: f32 = 1.12;
const PENALTY_LAST_N: i32 = 128;
const REPETITION_MIN_CHARS: usize = 40;
const REPETITION_TIMES: usize = 3;
/// Max tokens per `decode()` call — must match `with_n_batch` below.
/// Exceeding this in a single decode aborts the process (GGML_ASSERT).
const N_BATCH: u32 = 512;
const PROMPT_TOO_LONG_MSG: &str =
    "Savol yoki topilgan huquqiy matn juda uzun, qisqartiring";

/// Unload GGUF weights after this much idle time (no inference).
pub const IDLE_UNLOAD_AFTER: Duration = Duration::from_secs(10 * 60);
const IDLE_WATCH_INTERVAL: Duration = Duration::from_secs(30);
const WAKE_POLL_INTERVAL: Duration = Duration::from_millis(100);
const WAKE_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    EogToken,
    MaxTokens,
    RepetitionGuard,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EogToken => "eog_token",
            Self::MaxTokens => "max_tokens",
            Self::RepetitionGuard => "repetition_guard",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("LLM not initialized: {0}")]
    NotReady(String),
    #[error("Model path missing or invalid: {0}")]
    InvalidPath(String),
    #[error("LLM load failed: {0}")]
    Load(String),
    #[error("Generation failed: {0}")]
    Generate(String),
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub n_ctx: u32,
    pub n_threads: i32,
    pub max_tokens: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            n_ctx: 4096,
            n_threads: paths::default_thread_count(),
            max_tokens: 1024,
        }
    }
}

#[derive(Clone)]
enum TemplateSource {
    GgufEmbedded,
    ExplicitQwenChatMl,
}

impl TemplateSource {
    fn label(&self) -> &'static str {
        match self {
            Self::GgufEmbedded => "gguf_embedded_metadata",
            Self::ExplicitQwenChatMl => "explicit_qwen25_chatml",
        }
    }
}

struct LoadedLlm {
    model: LlamaModel,
    template: LlamaChatTemplate,
    template_source: TemplateSource,
    config: LlmConfig,
}

/// Shared LLM engine. Model load happens on a background thread (non-blocking).
pub struct LlmEngine {
    backend: Arc<LlamaBackend>,
    loaded: Arc<Mutex<Option<LoadedLlm>>>,
    loading: Arc<AtomicBool>,
    waking: Arc<AtomicBool>,
    idle_unloaded: Arc<AtomicBool>,
    inference_active: Arc<AtomicBool>,
    last_activity: Arc<Mutex<Instant>>,
    last_error: Arc<Mutex<Option<String>>>,
    model_path: Arc<Mutex<Option<PathBuf>>>,
    template_source_label: Arc<Mutex<Option<String>>>,
    config: Arc<Mutex<LlmConfig>>,
}

impl LlmEngine {
    pub fn new(config: LlmConfig) -> Result<Self, LlmError> {
        let backend = LlamaBackend::init().map_err(|e| LlmError::Load(e.to_string()))?;
        Ok(Self {
            backend: Arc::new(backend),
            loaded: Arc::new(Mutex::new(None)),
            loading: Arc::new(AtomicBool::new(false)),
            waking: Arc::new(AtomicBool::new(false)),
            idle_unloaded: Arc::new(AtomicBool::new(false)),
            inference_active: Arc::new(AtomicBool::new(false)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            last_error: Arc::new(Mutex::new(None)),
            model_path: Arc::new(Mutex::new(None)),
            template_source_label: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(config)),
        })
    }

    pub fn is_loading(&self) -> bool {
        self.loading.load(Ordering::SeqCst)
    }

    pub fn is_waking(&self) -> bool {
        self.waking.load(Ordering::SeqCst)
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn model_path(&self) -> Option<PathBuf> {
        self.model_path.lock().ok().and_then(|g| g.clone())
    }

    pub fn template_source(&self) -> Option<String> {
        self.template_source_label.lock().ok().and_then(|g| g.clone())
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|g| g.clone())
    }

    pub fn touch_activity(&self) {
        if let Ok(mut guard) = self.last_activity.lock() {
            *guard = Instant::now();
        }
    }

    pub fn idle_duration(&self) -> Duration {
        self.last_activity
            .lock()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    pub fn update_config(&self, config: LlmConfig) {
        if let Ok(mut guard) = self.config.lock() {
            *guard = config;
        }
    }

    /// Drop loaded weights but keep the configured GGUF path for lazy reload.
    pub fn unload(&self) {
        if self.inference_active.load(Ordering::SeqCst) || self.is_loading() {
            return;
        }
        let mut dropped = false;
        if let Ok(mut guard) = self.loaded.lock() {
            if guard.take().is_some() {
                dropped = true;
            }
        }
        if dropped {
            self.idle_unloaded.store(true, Ordering::SeqCst);
            info!(
                idle_secs = self.idle_duration().as_secs(),
                "Unloaded GGUF from memory after idle timeout"
            );
        }
    }

    /// Start a background watcher that unloads the model after [`IDLE_UNLOAD_AFTER`].
    pub fn spawn_idle_unload_watcher(self: &Arc<Self>) {
        let engine = Arc::clone(self);
        thread::Builder::new()
            .name("faqih-llm-idle".into())
            .spawn(move || loop {
                thread::sleep(IDLE_WATCH_INTERVAL);
                if !engine.is_loaded() {
                    continue;
                }
                if engine.inference_active.load(Ordering::SeqCst) || engine.is_loading() {
                    continue;
                }
                if engine.idle_duration() >= IDLE_UNLOAD_AFTER {
                    engine.unload();
                }
            })
            .ok();
    }

    /// Ensure the GGUF is resident. Reloads lazily after idle-unload and blocks until ready.
    pub fn ensure_loaded_for_inference(&self) -> Result<(), LlmError> {
        self.touch_activity();
        if self.is_loaded() {
            self.waking.store(false, Ordering::SeqCst);
            return Ok(());
        }

        let path = self.model_path().ok_or_else(|| {
            LlmError::NotReady("No GGUF model loaded. Choose a local model in Settings.".into())
        })?;

        let from_idle = self.idle_unloaded.load(Ordering::SeqCst);
        if from_idle {
            self.waking.store(true, Ordering::SeqCst);
        }
        if !self.is_loading() {
            if from_idle {
                info!(path = %path.display(), "Lazy-reloading GGUF after idle unload");
            }
            self.load_model_async(&path);
        }

        let deadline = Instant::now() + WAKE_TIMEOUT;
        while Instant::now() < deadline {
            if self.is_loaded() {
                self.waking.store(false, Ordering::SeqCst);
                self.idle_unloaded.store(false, Ordering::SeqCst);
                self.touch_activity();
                return Ok(());
            }
            if !self.is_loading() {
                self.waking.store(false, Ordering::SeqCst);
                return Err(LlmError::Load(
                    self.last_error()
                        .unwrap_or_else(|| "Model failed to load".into()),
                ));
            }
            thread::sleep(WAKE_POLL_INTERVAL);
        }

        self.waking.store(false, Ordering::SeqCst);
        Err(LlmError::Load(
            "Timed out waiting for GGUF to finish loading".into(),
        ))
    }

    /// Kick off a non-blocking background load of the GGUF at `path`.
    pub fn load_model_async(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            if let Ok(mut err) = self.last_error.lock() {
                *err = Some(format!("Model path does not exist: {}", path.display()));
            }
            self.waking.store(false, Ordering::SeqCst);
            return;
        }

        if self.loading.swap(true, Ordering::SeqCst) {
            return;
        }

        // Persist desired path immediately so idle-unload can reload later.
        if let Ok(mut guard) = self.model_path.lock() {
            *guard = Some(path.clone());
        }
        self.touch_activity();

        let backend = Arc::clone(&self.backend);
        let loaded = Arc::clone(&self.loaded);
        let loading = Arc::clone(&self.loading);
        let waking = Arc::clone(&self.waking);
        let idle_unloaded = Arc::clone(&self.idle_unloaded);
        let last_error = Arc::clone(&self.last_error);
        let model_path = Arc::clone(&self.model_path);
        let template_source_label = Arc::clone(&self.template_source_label);
        let config = Arc::clone(&self.config);
        let last_activity = Arc::clone(&self.last_activity);

        thread::spawn(move || {
            let result = (|| -> Result<LoadedLlm, LlmError> {
                let cfg = config
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_default();
                info!(path = %path.display(), n_ctx = cfg.n_ctx, "Loading GGUF model");
                let model_params = LlamaModelParams::default();
                let model = LlamaModel::load_from_file(backend.as_ref(), &path, &model_params)
                    .map_err(|e| LlmError::Load(e.to_string()))?;

                let (template, source) = match model.chat_template(None) {
                    Ok(tmpl) => {
                        info!(
                            "Chat template: using GGUF-embedded metadata via llama-cpp-2 chat_template()"
                        );
                        (tmpl, TemplateSource::GgufEmbedded)
                    }
                    Err(err) => {
                        warn!(
                            error = %err,
                            "Chat template: GGUF metadata missing/unreadable; using explicit Qwen2.5 ChatML"
                        );
                        let tmpl = LlamaChatTemplate::new(QWEN_CHATML_JINJA)
                            .map_err(|e| LlmError::Load(e.to_string()))?;
                        (tmpl, TemplateSource::ExplicitQwenChatMl)
                    }
                };

                Ok(LoadedLlm {
                    model,
                    template,
                    template_source: source,
                    config: cfg,
                })
            })();

            match result {
                Ok(llm) => {
                    let label = llm.template_source.label().to_string();
                    if let Ok(mut guard) = template_source_label.lock() {
                        *guard = Some(label);
                    }
                    if let Ok(mut guard) = model_path.lock() {
                        *guard = Some(path);
                    }
                    if let Ok(mut guard) = last_error.lock() {
                        *guard = None;
                    }
                    if let Ok(mut guard) = loaded.lock() {
                        *guard = Some(llm);
                    }
                    if let Ok(mut guard) = last_activity.lock() {
                        *guard = Instant::now();
                    }
                    idle_unloaded.store(false, Ordering::SeqCst);
                    info!("GGUF model loaded successfully");
                }
                Err(err) => {
                    warn!(error = %err, "GGUF model load failed");
                    if let Ok(mut guard) = last_error.lock() {
                        *guard = Some(err.to_string());
                    }
                    if let Ok(mut guard) = loaded.lock() {
                        *guard = None;
                    }
                }
            }
            waking.store(false, Ordering::SeqCst);
            loading.store(false, Ordering::SeqCst);
        });
    }

    pub fn format_chat(&self, system: &str, user: &str) -> Result<String, LlmError> {
        let guard = self
            .loaded
            .lock()
            .map_err(|_| LlmError::NotReady("lock poisoned".into()))?;
        let llm = guard
            .as_ref()
            .ok_or_else(|| LlmError::NotReady("model not loaded".into()))?;

        let messages = vec![
            LlamaChatMessage::new("system".into(), system.into())
                .map_err(|e| LlmError::Generate(e.to_string()))?,
            LlamaChatMessage::new("user".into(), user.into())
                .map_err(|e| LlmError::Generate(e.to_string()))?,
        ];

        match llm.template_source {
            TemplateSource::GgufEmbedded => llm
                .model
                .apply_chat_template(&llm.template, &messages, true)
                .map_err(|e| LlmError::Generate(e.to_string())),
            TemplateSource::ExplicitQwenChatMl => Ok(format_qwen_chatml(system, user)),
        }
    }

    /// Stream tokens. Callback is `FnMut(&str) + Send + 'static`.
    /// Final answer is read from a shared `Arc<Mutex<String>>` accumulator (not closure-local).
    pub fn generate_stream<F>(
        &self,
        prompt: &str,
        max_tokens: Option<u32>,
        on_token: F,
    ) -> Result<(String, StopReason), LlmError>
    where
        F: FnMut(&str) + Send + 'static,
    {
        self.touch_activity();
        self.inference_active.store(true, Ordering::SeqCst);

        let accumulator = Arc::new(Mutex::new(String::new()));
        let acc_for_callback = Arc::clone(&accumulator);
        let mut on_token = on_token;

        let mut callback = move |piece: &str| {
            if let Ok(mut guard) = acc_for_callback.lock() {
                guard.push_str(piece);
            }
            on_token(piece);
        };

        let result = self.generate_stream_inner(prompt, max_tokens, &mut callback);
        self.inference_active.store(false, Ordering::SeqCst);
        self.touch_activity();

        let stop = result?;
        let answer = accumulator
            .lock()
            .map_err(|_| LlmError::Generate("accumulator lock poisoned".into()))?
            .clone();
        Ok((answer, stop))
    }

    fn generate_stream_inner(
        &self,
        prompt: &str,
        max_tokens: Option<u32>,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<StopReason, LlmError> {
        let mut guard = self
            .loaded
            .lock()
            .map_err(|_| LlmError::NotReady("lock poisoned".into()))?;
        let llm = guard
            .as_mut()
            .ok_or_else(|| LlmError::NotReady("model not loaded".into()))?;

        let max_tokens = max_tokens.unwrap_or(llm.config.max_tokens) as usize;
        let n_ctx_u32 = llm.config.n_ctx.max(N_BATCH);
        let n_ctx_limit = n_ctx_u32 as usize;
        let n_ctx = NonZeroU32::new(n_ctx_u32).unwrap_or(NonZeroU32::new(4096).unwrap());
        let n_batch = N_BATCH as usize;
        // CRITICAL: with_n_ctx takes Option<NonZeroU32> — always wrap with Some(...)
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(n_ctx))
            .with_n_batch(N_BATCH)
            .with_n_threads(llm.config.n_threads)
            .with_n_threads_batch(llm.config.n_threads);

        let mut ctx = llm
            .model
            .new_context(self.backend.as_ref(), ctx_params)
            .map_err(|e| LlmError::Generate(e.to_string()))?;
        ctx.clear_kv_cache();

        let tokens = llm
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| LlmError::Generate(e.to_string()))?;
        if tokens.is_empty() {
            return Err(LlmError::Generate("prompt produced zero tokens".into()));
        }

        // Soft guard: never let oversized prompts hit GGML_ASSERT / hard abort.
        if tokens.len() > n_ctx_limit {
            warn!(
                prompt_tokens = tokens.len(),
                n_ctx = n_ctx_limit,
                "Prompt exceeds context window; refusing generation"
            );
            return Err(LlmError::Generate(PROMPT_TOO_LONG_MSG.into()));
        }

        let n_prompt = tokens.len();
        let n_chunks = prompt_decode_chunks(n_prompt, n_batch);
        info!(
            "[faqih] prompt has {n_prompt} tokens, processing in {n_chunks} chunk(s) of <={n_batch}"
        );

        // Capacity = n_batch (not full prompt): each decode() must stay ≤ n_batch.
        let mut batch = LlamaBatch::new(n_batch.max(1), 1);
        for (chunk_idx, chunk) in tokens.chunks(n_batch).enumerate() {
            let is_last_chunk = chunk_idx + 1 == n_chunks;
            batch.clear();
            for (offset, token) in chunk.iter().enumerate() {
                let abs_pos = chunk_idx * n_batch + offset;
                let need_logits = is_last_chunk && offset + 1 == chunk.len();
                batch
                    .add(
                        *token,
                        i32::try_from(abs_pos).unwrap_or(0),
                        &[0],
                        need_logits,
                    )
                    .map_err(|e| LlmError::Generate(e.to_string()))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| LlmError::Generate(e.to_string()))?;
        }

        let n_vocab = llm.model.n_vocab();
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(n_vocab, PENALTY_LAST_N, PENALTY_REPEAT, 0.0, 0.0),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::temp(0.7),
            LlamaSampler::dist(42),
        ]);

        let mut decoder = UTF_8.new_decoder();
        // Absolute KV position for the next generated token (= prompt length).
        // Do NOT use batch.n_tokens() here — after chunked prefill that is only
        // the last chunk size, not the full prompt length.
        let mut n_cur = i32::try_from(n_prompt).unwrap_or(i32::MAX);
        let mut generated = String::new();

        for _ in 0..max_tokens {
            if (n_cur as usize) >= n_ctx_limit {
                warn!(
                    n_cur,
                    n_ctx = n_ctx_limit,
                    "Context filled during generation; stopping early"
                );
                return Ok(StopReason::MaxTokens);
            }

            // CRITICAL: sample using batch.n_tokens() - 1, never a hardcoded index
            let logit_idx = batch.n_tokens() - 1;
            let token = sampler.sample(&ctx, logit_idx);
            sampler.accept(token);

            if llm.model.is_eog_token(token) {
                info!(stop = "eog_token", "Generation stopped: EOG token");
                return Ok(StopReason::EogToken);
            }

            // CRITICAL: token_to_piece (not deprecated token_to_bytes)
            let piece = llm
                .model
                .token_to_piece(token, &mut decoder, false, None::<NonZeroU16>)
                .unwrap_or_default();

            if !piece.is_empty() {
                generated.push_str(&piece);
                on_token(&piece);
            }

            if repetition_guard_triggered(&generated) {
                info!(
                    stop = "repetition_guard",
                    "Generation stopped: 40+ char substring repeated {}x consecutively",
                    REPETITION_TIMES
                );
                return Ok(StopReason::RepetitionGuard);
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| LlmError::Generate(e.to_string()))?;
            ctx.decode(&mut batch)
                .map_err(|e| LlmError::Generate(e.to_string()))?;
            n_cur += 1;
        }

        info!(stop = "max_tokens", max_tokens, "Generation stopped: max_tokens");
        Ok(StopReason::MaxTokens)
    }

    pub fn generate(&self, prompt: &str, max_tokens: Option<u32>) -> Result<(String, StopReason), LlmError> {
        self.generate_stream(prompt, max_tokens, |_piece| {})
    }

    pub fn n_ctx(&self) -> u32 {
        self.config.lock().map(|c| c.n_ctx).unwrap_or(4096)
    }
}

/// Explicit Qwen2.5-Instruct ChatML (used when GGUF metadata has no template).
fn format_qwen_chatml(system: &str, user: &str) -> String {
    format!(
        "<|im_start|>system\n{system}<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
    )
}

/// How many decode() chunks a prompt of `n_prompt` tokens needs at `n_batch`.
fn prompt_decode_chunks(n_prompt: usize, n_batch: usize) -> usize {
    if n_prompt == 0 || n_batch == 0 {
        return 0;
    }
    n_prompt.div_ceil(n_batch)
}

/// Minimal jinja chat template string recognized by llama.cpp's template engine as ChatML-like.
/// Used only when constructing LlamaChatTemplate as a named/fallback template handle.
const QWEN_CHATML_JINJA: &str = r#"{% for message in messages %}{{'<|im_start|>' + message['role'] + '\n' + message['content'] + '<|im_end|>' + '\n'}}{% endfor %}{% if add_generation_prompt %}{{ '<|im_start|>assistant\n' }}{% endif %}"#;

fn repetition_guard_triggered(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if n < REPETITION_MIN_CHARS * REPETITION_TIMES {
        return false;
    }

    let max_unit = (n / REPETITION_TIMES).min(240);
    for unit_len in REPETITION_MIN_CHARS..=max_unit {
        let start = n - unit_len * REPETITION_TIMES;
        let unit: String = chars[n - unit_len..].iter().collect();
        let window: String = chars[start..].iter().collect();
        if window == unit.repeat(REPETITION_TIMES) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repetition_guard_detects_long_loops() {
        let unit = "A".repeat(40);
        let text = unit.repeat(3);
        assert!(repetition_guard_triggered(&text));
    }

    #[test]
    fn repetition_guard_ignores_short_legal_terms() {
        let text = "modda ".repeat(20);
        assert!(!repetition_guard_triggered(&text));
    }

    #[test]
    fn prompt_chunking_splits_above_n_batch() {
        assert_eq!(prompt_decode_chunks(512, 512), 1);
        assert_eq!(prompt_decode_chunks(513, 512), 2);
        assert_eq!(prompt_decode_chunks(743, 512), 2);
        assert_eq!(prompt_decode_chunks(1024, 512), 2);
        assert_eq!(prompt_decode_chunks(1025, 512), 3);
    }
}
