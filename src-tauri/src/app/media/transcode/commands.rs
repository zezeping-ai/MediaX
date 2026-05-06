use super::processor::{emit_estimate, schedule_jobs};
use super::state::{
    TranscodeJob, TranscodeJobKind, TranscodeJobStatus, TranscodeQueueSnapshot, TranscodeState,
};
use super::utils::next_available_output_path;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, State};

#[derive(Debug, serde::Serialize)]
pub struct VideoProbeResponse {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoTranscodeRequest {
    pub source_path: String,
    pub output_dir: String,
    pub format: String,
    pub resolution: String,
    pub playback_rate: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AudioTranscodeRequest {
    pub source_path: String,
    pub output_dir: String,
    pub format: String,
    pub playback_rate: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageCompressRequest {
    pub source_paths: Vec<String>,
    pub output_dir: Option<String>,
    pub mode: String,
    pub format: Option<String>,
    pub quality: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageCompressEstimateRequest {
    pub source_paths: Vec<String>,
    pub mode: String,
    pub format: Option<String>,
    pub quality: Option<u8>,
}

#[derive(Debug, serde::Serialize)]
pub struct ImageCompressEstimateResponse {
    pub total_input_size_bytes: u64,
    pub estimated_output_size_bytes: u64,
}

fn validate_path(value: &str, field_name: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} 不能为空"));
    }
    Ok(trimmed.to_string())
}

fn derive_extension_from_format(format: &str, fallback: &str) -> String {
    let normalized = format.trim().to_lowercase();
    if normalized.is_empty() {
        return fallback.to_string();
    }
    normalized
        .split('/')
        .next()
        .unwrap_or(fallback)
        .split('+')
        .next()
        .unwrap_or(fallback)
        .to_string()
}

fn estimate_lossy_ratio(format: &str, quality: u8) -> f64 {
    let q = quality.clamp(1, 100) as f64 / 100.0;
    match format {
        // Heuristic: JPEG usually scales roughly with quality.
        "jpeg" => (0.18 + q * 0.80).clamp(0.10, 0.98),
        // WebP tends to be smaller than JPEG at same visual quality.
        "webp" => (0.12 + q * 0.70).clamp(0.08, 0.95),
        // PNG in this pipeline uses re-encode path; keep a narrow quality-linked estimate range.
        "png" => (0.78 + q * 0.20).clamp(0.75, 0.98),
        "gif" => (0.25 + q * 0.65).clamp(0.20, 0.95),
        "bmp" => (0.90 + q * 0.08).clamp(0.90, 0.99),
        _ => (0.18 + q * 0.80).clamp(0.10, 0.98),
    }
}

#[tauri::command]
pub fn image_compress_estimate(payload: ImageCompressEstimateRequest) -> Result<ImageCompressEstimateResponse, String> {
    if payload.source_paths.is_empty() {
        return Ok(ImageCompressEstimateResponse {
            total_input_size_bytes: 0,
            estimated_output_size_bytes: 0,
        });
    }
    let mode = payload.mode.trim().to_lowercase();
    let format = payload
        .format
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("jpeg")
        .to_lowercase();
    let quality = payload.quality.unwrap_or(75).clamp(1, 100);
    if mode == "lossy"
        && format != "jpeg"
        && format != "webp"
        && format != "png"
        && format != "gif"
        && format != "bmp"
    {
        return Err("有损压缩格式仅支持 jpeg/webp/png/gif/bmp".to_string());
    }
    let mut total_input_size_bytes = 0_u64;
    for path in payload.source_paths {
        let normalized = validate_path(&path, "图片路径")?;
        if let Ok(meta) = fs::metadata(&normalized) {
            total_input_size_bytes = total_input_size_bytes.saturating_add(meta.len());
        }
    }
    let estimated_output_size_bytes = if mode == "lossy" {
        let ratio = estimate_lossy_ratio(&format, quality);
        (total_input_size_bytes as f64 * ratio) as u64
    } else {
        total_input_size_bytes.saturating_mul(90) / 100
    };
    Ok(ImageCompressEstimateResponse {
        total_input_size_bytes,
        estimated_output_size_bytes,
    })
}

#[tauri::command]
pub fn transcode_queue_snapshot(state: State<'_, TranscodeState>) -> Result<TranscodeQueueSnapshot, String> {
    Ok(state.snapshot())
}

#[tauri::command]
pub fn transcode_job_cancel(job_id: u64, state: State<'_, TranscodeState>) -> Result<TranscodeQueueSnapshot, String> {
    state.with_inner(|inner| {
        inner.canceled.push(job_id);
        inner.pending.retain(|id| *id != job_id);
        if let Some(job) = inner.jobs.get_mut(&job_id) {
            job.status = TranscodeJobStatus::Canceled;
            job.error_message = Some("任务已取消".to_string());
        }
    });
    Ok(state.snapshot())
}

#[tauri::command]
pub fn transcode_job_remove(job_id: u64, state: State<'_, TranscodeState>) -> Result<TranscodeQueueSnapshot, String> {
    state.with_inner(|inner| {
        inner.pending.retain(|id| *id != job_id);
        inner.running.retain(|id| *id != job_id);
        inner.canceled.retain(|id| *id != job_id);
        inner.jobs.remove(&job_id);
        inner.order.retain(|id| *id != job_id);
    });
    Ok(state.snapshot())
}

#[tauri::command]
pub fn transcode_video_enqueue(
    app: AppHandle,
    state: State<'_, TranscodeState>,
    payload: VideoTranscodeRequest,
) -> Result<TranscodeQueueSnapshot, String> {
    let source_path = validate_path(&payload.source_path, "视频源路径")?;
    let output_dir = validate_path(&payload.output_dir, "输出目录")?;
    fs::create_dir_all(&output_dir).map_err(|err| format!("创建输出目录失败: {err}"))?;
    let extension = derive_extension_from_format(&payload.format, "mp4");
    let output_path = next_available_output_path(
        &output_dir,
        &source_path,
        "-transcoded",
        &extension,
        "mediax-video",
    )
    .display()
    .to_string();
    let input_size = fs::metadata(&source_path).map(|meta| meta.len()).ok();
    let job_id = state.next_job_id();
    let job = TranscodeJob {
        id: job_id,
        kind: TranscodeJobKind::Video,
        source_path,
        output_path,
        status: TranscodeJobStatus::Queued,
        progress_percent: 0.0,
        started_at_ms: None,
        finished_at_ms: None,
        error_message: None,
        input_size_bytes: input_size,
        output_size_bytes: None,
        estimated_output_size_bytes: input_size,
        quality: None,
        format: Some(payload.format),
        resolution: Some(payload.resolution),
        playback_rate: Some(payload.playback_rate),
    };
    state.with_inner(|inner| {
        inner.order.push(job_id);
        inner.pending.push_back(job_id);
        inner.jobs.insert(job_id, job);
    });
    schedule_jobs(&app);
    Ok(state.snapshot())
}

#[tauri::command]
pub fn transcode_video_probe(source_path: String) -> Result<VideoProbeResponse, String> {
    let source_path = validate_path(&source_path, "视频源路径")?;
    let (width, height) = super::processor::probe_video_dimensions(&source_path)?;
    Ok(VideoProbeResponse { width, height })
}

#[tauri::command]
pub fn transcode_audio_enqueue(
    app: AppHandle,
    state: State<'_, TranscodeState>,
    payload: AudioTranscodeRequest,
) -> Result<TranscodeQueueSnapshot, String> {
    let source_path = validate_path(&payload.source_path, "音频源路径")?;
    let output_dir = validate_path(&payload.output_dir, "输出目录")?;
    fs::create_dir_all(&output_dir).map_err(|err| format!("创建输出目录失败: {err}"))?;
    let extension = derive_extension_from_format(&payload.format, "m4a");
    let output_path = next_available_output_path(
        &output_dir,
        &source_path,
        "-transcoded",
        &extension,
        "mediax-audio",
    )
    .display()
    .to_string();
    let input_size = fs::metadata(&source_path).map(|meta| meta.len()).ok();
    let job_id = state.next_job_id();
    let job = TranscodeJob {
        id: job_id,
        kind: TranscodeJobKind::Audio,
        source_path,
        output_path,
        status: TranscodeJobStatus::Queued,
        progress_percent: 0.0,
        started_at_ms: None,
        finished_at_ms: None,
        error_message: None,
        input_size_bytes: input_size,
        output_size_bytes: None,
        estimated_output_size_bytes: input_size,
        quality: None,
        format: Some(payload.format),
        resolution: None,
        playback_rate: Some(payload.playback_rate),
    };
    state.with_inner(|inner| {
        inner.order.push(job_id);
        inner.pending.push_back(job_id);
        inner.jobs.insert(job_id, job);
    });
    schedule_jobs(&app);
    Ok(state.snapshot())
}

#[tauri::command]
pub fn image_compress_enqueue(
    app: AppHandle,
    state: State<'_, TranscodeState>,
    payload: ImageCompressRequest,
) -> Result<TranscodeQueueSnapshot, String> {
    let configured_output_dir = payload
        .output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());
    if let Some(output_dir) = &configured_output_dir {
        fs::create_dir_all(output_dir).map_err(|err| format!("创建输出目录失败: {err}"))?;
    }
    let mode = payload.mode.trim().to_lowercase();
    let lossy_format = payload
        .format
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("jpeg")
        .to_lowercase();
    if mode == "lossy"
        && lossy_format != "jpeg"
        && lossy_format != "webp"
        && lossy_format != "png"
        && lossy_format != "gif"
        && lossy_format != "bmp"
    {
        return Err("有损压缩格式仅支持 jpeg/webp/png/gif/bmp".to_string());
    }
    if payload.source_paths.is_empty() {
        return Err("至少选择一张图片".to_string());
    }
    for source_path in payload.source_paths {
        let source_path = validate_path(&source_path, "图片路径")?;
        let output_dir = configured_output_dir.clone().unwrap_or_else(|| {
            Path::new(&source_path)
                .parent()
                .and_then(|path| path.to_str())
                .unwrap_or(".")
                .to_string()
        });
        fs::create_dir_all(&output_dir).map_err(|err| format!("创建输出目录失败: {err}"))?;
        let input_size = fs::metadata(&source_path).map(|meta| meta.len()).ok();
        let extension = if mode == "lossy" {
            if lossy_format == "webp" {
                "webp".to_string()
            } else if lossy_format == "png" {
                "png".to_string()
            } else if lossy_format == "gif" {
                "gif".to_string()
            } else if lossy_format == "bmp" {
                "bmp".to_string()
            } else {
                "jpg".to_string()
            }
        } else {
            std::path::Path::new(&source_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("png")
                .to_string()
        };
        let output_path = if mode == "lossless" {
            // Lossless mode overwrites source file in place by product requirement.
            source_path.clone()
        } else {
            next_available_output_path(
                &output_dir,
                &source_path,
                "-compressed",
                &extension,
                "mediax-image",
            )
            .display()
            .to_string()
        };
        let job_id = state.next_job_id();
        let kind = if mode == "lossy" {
            TranscodeJobKind::ImageLossy
        } else {
            TranscodeJobKind::ImageLossless
        };
        let estimated_output_size_bytes = input_size.map(|size| {
            if mode == "lossy" {
                let ratio = estimate_lossy_ratio(&lossy_format, payload.quality.unwrap_or(75));
                (size as f64 * ratio) as u64
            } else {
                size.saturating_mul(90) / 100
            }
        });
        let job = TranscodeJob {
            id: job_id,
            kind,
            source_path,
            output_path,
            status: TranscodeJobStatus::Queued,
            progress_percent: 0.0,
            started_at_ms: None,
            finished_at_ms: None,
            error_message: None,
            input_size_bytes: input_size,
            output_size_bytes: None,
            estimated_output_size_bytes,
            quality: payload.quality,
            format: if mode == "lossy" {
                Some(lossy_format.clone())
            } else {
                None
            },
            resolution: None,
            playback_rate: None,
        };
        state.with_inner(|inner| {
            inner.order.push(job_id);
            inner.pending.push_back(job_id);
            inner.jobs.insert(job_id, job);
        });
        if let Some(estimate) = estimated_output_size_bytes {
            emit_estimate(&app, job_id, estimate);
        }
    }
    schedule_jobs(&app);
    Ok(state.snapshot())
}
