//! First-run download of the bundled AI model (user-facing: "AI modeli").
//!
//! Downloads Qwen2.5-1.5B-Instruct Q4_K_M into the app data directory with
//! Range resume, size verification, and progress events for the UI.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tracing::{info, warn};

use crate::models::ModelStatus;
use crate::paths;
use crate::state::AppState;

/// Hugging Face resolve URL (follows CDN redirect). Filename matches bartowski release.
pub const MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/bartowski/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf";

pub const MODEL_FILENAME: &str = "Qwen2.5-1.5B-Instruct-Q4_K_M.gguf";

/// Exact Content-Length observed from Hugging Face CDN (2026-09-10).
pub const EXPECTED_MODEL_BYTES: u64 = 986_048_768;

pub const MODEL_DOWNLOAD_PROGRESS_EVENT: &str = "model-download-progress";

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
const PROGRESS_EMIT_EVERY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSetupStatus {
    pub ready: bool,
    pub downloading: bool,
    pub expected_bytes: u64,
    pub local_path: Option<String>,
    pub partial_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: f64,
    pub eta_seconds: Option<u64>,
    pub phase: String,
    pub message: Option<String>,
}

static DOWNLOAD_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

pub fn default_model_path() -> PathBuf {
    paths::app_data_dir().join("models").join(MODEL_FILENAME)
}

pub fn partial_model_path() -> PathBuf {
    paths::app_data_dir()
        .join("models")
        .join(format!("{MODEL_FILENAME}.partial"))
}

pub fn is_valid_model_file(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if meta.len() != EXPECTED_MODEL_BYTES {
        return false;
    }
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut magic = [0u8; 4];
    match file.read_exact(&mut magic) {
        Ok(()) => &magic == GGUF_MAGIC,
        Err(_) => false,
    }
}

/// If settings have no usable model but the default download exists, wire it up.
pub fn hydrate_settings_from_default_model(settings: &mut crate::models::AppSettings) -> bool {
    let path_ok = settings
        .model_path
        .as_ref()
        .map(|p| Path::new(p).is_file() && is_valid_model_file(Path::new(p)))
        .unwrap_or(false);
    if path_ok {
        return false;
    }

    let default = default_model_path();
    if is_valid_model_file(&default) {
        settings.model_path = Some(default.display().to_string());
        let _ = paths::save_settings(settings);
        info!(path = %default.display(), "Hydrated settings from previously downloaded AI model");
        return true;
    }
    false
}

fn friendly_error(kind: &str, detail: impl AsRef<str>) -> String {
    let detail = detail.as_ref();
    match kind {
        "network" => format!(
            "Internetga ulanishda muammo yuz berdi. Iltimos, aloqani tekshirib, qayta urinib ko‘ring.\n({detail})"
        ),
        "disk" => format!(
            "Diskda yetarli joy yo‘q yoki faylni yozib bo‘lmadi. Bo‘sh joy ajratib, qayta urinib ko‘ring.\n({detail})"
        ),
        "verify" => format!(
            "Yuklab olingan AI modeli tekshiruvdan o‘tmadi. Qayta yuklab olish kerak.\n({detail})"
        ),
        "busy" => "Model yuklab olinmoqda. Iltimos, kuting.".into(),
        _ => format!("Modelni yuklab olishda xatolik. Qayta urinib ko‘ring.\n({detail})"),
    }
}

fn emit_progress(app: &AppHandle, progress: ModelDownloadProgress) {
    let _ = app.emit(MODEL_DOWNLOAD_PROGRESS_EVENT, progress);
}

fn ensure_models_dir() -> Result<PathBuf, String> {
    let dir = paths::app_data_dir().join("models");
    fs::create_dir_all(&dir).map_err(|e| friendly_error("disk", e.to_string()))?;
    Ok(dir)
}

#[tauri::command]
pub fn get_model_setup_status(state: State<'_, AppState>) -> Result<ModelSetupStatus, String> {
    let settings = state.settings.lock().clone();
    let configured = settings.model_path.as_ref().and_then(|p| {
        let path = Path::new(p);
        if path.is_file() && is_valid_model_file(path) {
            Some(p.clone())
        } else {
            None
        }
    });
    let default = default_model_path();
    let ready_path = configured.or_else(|| {
        if is_valid_model_file(&default) {
            Some(default.display().to_string())
        } else {
            None
        }
    });

    let partial = partial_model_path();
    let partial_bytes = fs::metadata(&partial).map(|m| m.len()).unwrap_or(0);

    Ok(ModelSetupStatus {
        ready: ready_path.is_some(),
        downloading: DOWNLOAD_IN_PROGRESS.load(Ordering::SeqCst),
        expected_bytes: EXPECTED_MODEL_BYTES,
        local_path: ready_path,
        partial_bytes,
    })
}

#[tauri::command]
pub async fn download_default_model(
    app: AppHandle,
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<ModelStatus, String> {
    if DOWNLOAD_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Err(friendly_error("busy", "already running"));
    }

    let force = force.unwrap_or(false);
    let result = download_default_model_inner(&app, force).await;
    DOWNLOAD_IN_PROGRESS.store(false, Ordering::SeqCst);

    match result {
        Ok(path) => {
            {
                let mut settings = state.settings.lock();
                settings.model_path = Some(path.clone());
                paths::save_settings(&settings)?;
            }
            state.llm.load_model_async(PathBuf::from(&path));
            emit_progress(
                &app,
                ModelDownloadProgress {
                    downloaded_bytes: EXPECTED_MODEL_BYTES,
                    total_bytes: EXPECTED_MODEL_BYTES,
                    bytes_per_second: 0.0,
                    eta_seconds: Some(0),
                    phase: "done".into(),
                    message: Some("AI modeli tayyor".into()),
                },
            );
            crate::commands::settings::get_model_status(state)
        }
        Err(err) => {
            emit_progress(
                &app,
                ModelDownloadProgress {
                    downloaded_bytes: 0,
                    total_bytes: EXPECTED_MODEL_BYTES,
                    bytes_per_second: 0.0,
                    eta_seconds: None,
                    phase: "error".into(),
                    message: Some(err.clone()),
                },
            );
            Err(err)
        }
    }
}

async fn download_default_model_inner(app: &AppHandle, force: bool) -> Result<String, String> {
    ensure_models_dir()?;
    let final_path = default_model_path();
    let partial_path = partial_model_path();

    if !force && is_valid_model_file(&final_path) {
        info!(path = %final_path.display(), "Default AI model already present");
        return Ok(final_path.display().to_string());
    }

    // Incomplete/corrupt final file, or forced re-download → remove and (re)download.
    if final_path.exists() {
        warn!(path = %final_path.display(), force, "Removing existing model file before download");
        let _ = fs::remove_file(&final_path);
    }
    if force {
        let _ = fs::remove_file(&partial_path);
    }

    emit_progress(
        app,
        ModelDownloadProgress {
            downloaded_bytes: 0,
            total_bytes: EXPECTED_MODEL_BYTES,
            bytes_per_second: 0.0,
            eta_seconds: None,
            phase: "starting".into(),
            message: Some("Yuklab olish boshlanmoqda…".into()),
        },
    );

    let client = reqwest::Client::builder()
        .user_agent("FaqihAI/0.1 (+offline legal assistant; first-run model download)")
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60 * 60))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| friendly_error("network", e.to_string()))?;

    let mut existing = if partial_path.exists() {
        fs::metadata(&partial_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    if existing >= EXPECTED_MODEL_BYTES {
        // Stale oversized partial — restart cleanly.
        let _ = fs::remove_file(&partial_path);
        existing = 0;
    }

    let mut request = client.get(MODEL_DOWNLOAD_URL);
    if existing > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={existing}-"));
        info!(resume_from = existing, "Resuming partial AI model download");
    }

    let response = request
        .send()
        .await
        .map_err(|e| friendly_error("network", e.to_string()))?;

    let status = response.status();
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        let _ = fs::remove_file(&partial_path);
        existing = 0;
        return Box::pin(download_default_model_inner(app, false)).await;
    }

    if existing > 0 && status == reqwest::StatusCode::OK {
        // Server ignored Range — restart from scratch to avoid corruption.
        warn!("Server did not honor Range; restarting download from zero");
        let _ = fs::remove_file(&partial_path);
        existing = 0;
        return Box::pin(download_default_model_inner(app, false)).await;
    }

    if !(status.is_success()
        || (existing > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT))
    {
        return Err(friendly_error(
            "network",
            format!("server status {status}"),
        ));
    }

    let total_from_headers = response
        .content_length()
        .map(|len| {
            if status == reqwest::StatusCode::PARTIAL_CONTENT {
                existing + len
            } else {
                len
            }
        })
        .unwrap_or(EXPECTED_MODEL_BYTES);

    // Prefer our known size; tolerate CDN reporting the same value.
    let total_bytes = if (total_from_headers as i64 - EXPECTED_MODEL_BYTES as i64).abs() < 1024 {
        EXPECTED_MODEL_BYTES
    } else if total_from_headers > 0 {
        total_from_headers
    } else {
        EXPECTED_MODEL_BYTES
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(existing > 0)
        .write(true)
        .truncate(existing == 0)
        .open(&partial_path)
        .map_err(|e| friendly_error("disk", e.to_string()))?;

    if existing > 0 {
        file.seek(SeekFrom::End(0))
            .map_err(|e| friendly_error("disk", e.to_string()))?;
    }

    let mut downloaded = existing;
    let mut stream = response.bytes_stream();
    let started = Instant::now();
    let mut last_emit = Instant::now() - PROGRESS_EMIT_EVERY;
    let mut window_start = Instant::now();
    let mut window_bytes: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| friendly_error("network", e.to_string()))?;
        file.write_all(&chunk)
            .map_err(|e| friendly_error("disk", e.to_string()))?;
        let n = chunk.len() as u64;
        downloaded += n;
        window_bytes += n;

        if last_emit.elapsed() >= PROGRESS_EMIT_EVERY || downloaded >= total_bytes {
            let elapsed = window_start.elapsed().as_secs_f64().max(0.001);
            let bps = window_bytes as f64 / elapsed;
            let remaining = total_bytes.saturating_sub(downloaded);
            let eta = if bps > 1.0 {
                Some((remaining as f64 / bps).ceil() as u64)
            } else {
                None
            };
            emit_progress(
                app,
                ModelDownloadProgress {
                    downloaded_bytes: downloaded.min(total_bytes),
                    total_bytes,
                    bytes_per_second: bps,
                    eta_seconds: eta,
                    phase: "downloading".into(),
                    message: None,
                },
            );
            last_emit = Instant::now();
            window_start = Instant::now();
            window_bytes = 0;
        }
    }

    file.flush()
        .map_err(|e| friendly_error("disk", e.to_string()))?;
    drop(file);

    let final_len = fs::metadata(&partial_path)
        .map(|m| m.len())
        .map_err(|e| friendly_error("disk", e.to_string()))?;

    if final_len != EXPECTED_MODEL_BYTES {
        let _ = fs::remove_file(&partial_path);
        return Err(friendly_error(
            "verify",
            format!("size {final_len}, expected {EXPECTED_MODEL_BYTES}"),
        ));
    }

    emit_progress(
        app,
        ModelDownloadProgress {
            downloaded_bytes: EXPECTED_MODEL_BYTES,
            total_bytes: EXPECTED_MODEL_BYTES,
            bytes_per_second: 0.0,
            eta_seconds: Some(0),
            phase: "verifying".into(),
            message: Some("Tekshirilmoqda…".into()),
        },
    );

    fs::rename(&partial_path, &final_path).map_err(|e| friendly_error("disk", e.to_string()))?;

    if !is_valid_model_file(&final_path) {
        let _ = fs::remove_file(&final_path);
        return Err(friendly_error("verify", "magic/size check failed"));
    }

    info!(
        path = %final_path.display(),
        elapsed_secs = started.elapsed().as_secs(),
        "AI model downloaded and verified"
    );

    Ok(final_path.display().to_string())
}

/// Soft cancel flag reserved for future UI; currently unused.
#[allow(dead_code)]
pub fn download_busy() -> bool {
    DOWNLOAD_IN_PROGRESS.load(Ordering::SeqCst)
}

pub fn model_needs_setup(settings: &crate::models::AppSettings) -> bool {
    let configured_ok = settings
        .model_path
        .as_ref()
        .map(|p| Path::new(p).is_file() && is_valid_model_file(Path::new(p)))
        .unwrap_or(false);
    if configured_ok {
        return false;
    }
    !is_valid_model_file(&default_model_path())
}
