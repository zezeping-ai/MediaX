use super::state::{TranscodeJob, TranscodeJobKind, TranscodeJobStatus, TranscodeState};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ColorType, ImageEncoder, ImageFormat};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use webp::Encoder as WebpEncoder;

pub const TRANSCODE_STATUS_EVENT: &str = "media://transcode/status";
pub const TRANSCODE_PROGRESS_EVENT: &str = "media://transcode/progress";
pub const TRANSCODE_ESTIMATE_EVENT: &str = "media://transcode/estimate";

#[derive(Serialize)]
pub struct TranscodeProgressPayload {
    pub job_id: u64,
    pub progress_percent: f64,
    pub status: TranscodeJobStatus,
    pub output_size_bytes: Option<u64>,
    pub error_message: Option<String>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn schedule_jobs(app: &AppHandle) {
    let state = app.state::<TranscodeState>();
    let mut to_start = Vec::new();
    state.with_inner(|inner| {
        while inner.running.len() < TranscodeState::MAX_CONCURRENT_JOBS {
            let Some(job_id) = inner.pending.pop_front() else {
                break;
            };
            if inner.canceled.contains(&job_id) {
                continue;
            }
            if let Some(job) = inner.jobs.get_mut(&job_id) {
                job.status = TranscodeJobStatus::Running;
                job.started_at_ms = Some(now_ms());
                job.progress_percent = 0.0;
                inner.running.push(job_id);
                to_start.push(job_id);
            }
        }
    });
    for job_id in to_start {
        let app_handle = app.clone();
        tauri::async_runtime::spawn(async move {
            let runner = app_handle.clone();
            let _ = tauri::async_runtime::spawn_blocking(move || {
                run_job(&runner, job_id);
            })
            .await;
            schedule_jobs(&app_handle);
        });
    }
}

fn emit_progress(app: &AppHandle, payload: TranscodeProgressPayload) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit(TRANSCODE_PROGRESS_EVENT, &payload);
        let _ = app.emit(TRANSCODE_STATUS_EVENT, &payload);
    });
}

fn update_progress(
    app: &AppHandle,
    job_id: u64,
    progress_percent: f64,
    output_size_bytes: Option<u64>,
    status: Option<TranscodeJobStatus>,
    error_message: Option<String>,
) {
    let state = app.state::<TranscodeState>();
    let mut current_status = TranscodeJobStatus::Running;
    state.with_inner(|inner| {
        if let Some(job) = inner.jobs.get_mut(&job_id) {
            job.progress_percent = progress_percent.clamp(0.0, 100.0);
            if let Some(bytes) = output_size_bytes {
                job.output_size_bytes = Some(bytes);
            }
            if let Some(next) = status.clone() {
                job.status = next;
            }
            if error_message.is_some() {
                job.error_message = error_message.clone();
            }
            current_status = job.status.clone();
        }
    });
    emit_progress(
        app,
        TranscodeProgressPayload {
            job_id,
            progress_percent,
            status: current_status,
            output_size_bytes,
            error_message,
        },
    );
}

fn run_job(app: &AppHandle, job_id: u64) {
    let job = {
        let state = app.state::<TranscodeState>();
        state.with_inner(|inner| inner.jobs.get(&job_id).cloned())
    };
    let Some(job) = job else {
        return;
    };
    let canceled = {
        let state = app.state::<TranscodeState>();
        state.with_inner(|inner| inner.canceled.contains(&job_id))
    };
    if canceled {
        finish_job(app, job_id, TranscodeJobStatus::Canceled, None);
        return;
    }
    let result = match job.kind {
        TranscodeJobKind::Video | TranscodeJobKind::Audio => copy_with_progress(app, &job),
        TranscodeJobKind::ImageLossless => compress_image_lossless(app, &job),
        TranscodeJobKind::ImageLossy => compress_image_lossy(app, &job),
    };
    match result {
        Ok(JobResult::Success(output_size_bytes)) => {
            update_progress(
                app,
                job_id,
                100.0,
                Some(output_size_bytes),
                Some(TranscodeJobStatus::Success),
                None,
            );
            finish_job(app, job_id, TranscodeJobStatus::Success, None);
        }
        Ok(JobResult::Skipped(output_size_bytes, reason)) => {
            update_progress(
                app,
                job_id,
                100.0,
                Some(output_size_bytes),
                Some(TranscodeJobStatus::Skipped),
                Some(reason.clone()),
            );
            finish_job(app, job_id, TranscodeJobStatus::Skipped, Some(reason));
        }
        Err(err) => {
            finish_job(app, job_id, TranscodeJobStatus::Failed, Some(err));
        }
    }
}

fn finish_job(app: &AppHandle, job_id: u64, status: TranscodeJobStatus, error_message: Option<String>) {
    let state = app.state::<TranscodeState>();
    let mut final_progress_percent = 0.0_f64;
    state.with_inner(|inner| {
        inner.running.retain(|id| *id != job_id);
        if let Some(job) = inner.jobs.get_mut(&job_id) {
            job.status = status.clone();
            job.finished_at_ms = Some(now_ms());
            if error_message.is_some() {
                job.error_message = error_message.clone();
            }
            if matches!(status, TranscodeJobStatus::Success) {
                job.progress_percent = 100.0;
            }
            final_progress_percent = job.progress_percent;
        }
    });
    emit_progress(
        app,
        TranscodeProgressPayload {
            job_id,
            progress_percent: final_progress_percent,
            status,
            output_size_bytes: None,
            error_message,
        },
    );
}

enum JobResult {
    Success(u64),
    Skipped(u64, String),
}

fn copy_with_progress(app: &AppHandle, job: &TranscodeJob) -> Result<JobResult, String> {
    if let Some(rate) = job.playback_rate {
        if (rate - 1.0).abs() > f64::EPSILON {
            return Err("当前阶段仅支持 1.0 倍速".to_string());
        }
    }
    if let Some(resolution) = &job.resolution {
        if resolution != "source" {
            return Err("当前阶段仅支持源分辨率".to_string());
        }
    }
    let source_path = Path::new(&job.source_path);
    let output_path = Path::new(&job.output_path);
    let mut source = File::open(source_path).map_err(|err| format!("打开输入文件失败: {err}"))?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建输出目录失败: {err}"))?;
    }
    let mut target = File::create(output_path).map_err(|err| format!("创建输出文件失败: {err}"))?;
    let total_size = source.metadata().map_err(|err| format!("读取输入文件元数据失败: {err}"))?.len();
    let mut written_size = 0_u64;
    let mut buf = vec![0_u8; 512 * 1024];
    loop {
        let read = source
            .read(&mut buf)
            .map_err(|err| format!("读取输入文件失败: {err}"))?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buf[..read])
            .map_err(|err| format!("写入输出文件失败: {err}"))?;
        written_size += read as u64;
        let progress = if total_size == 0 {
            100.0
        } else {
            (written_size as f64 / total_size as f64) * 100.0
        };
        update_progress(
            app,
            job.id,
            progress.min(99.0),
            Some(written_size),
            None,
            None,
        );
    }
    Ok(JobResult::Success(written_size))
}

fn compress_image_lossless(app: &AppHandle, job: &TranscodeJob) -> Result<JobResult, String> {
    let source_path = Path::new(&job.source_path);
    let output_path = Path::new(&job.output_path);
    let input_size = fs::metadata(source_path)
        .map_err(|err| format!("读取输入文件元数据失败: {err}"))?
        .len();
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    update_progress(app, job.id, 12.0, None, None, None);

    update_progress(app, job.id, 28.0, None, None, None);
    let image = image::open(source_path).map_err(|err| format!("读取图片失败: {err}"))?;
    update_progress(app, job.id, 46.0, None, None, None);

    if extension == "png" {
        let mut encoded = Vec::new();
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();
        update_progress(app, job.id, 64.0, None, None, None);
        let mut cursor = Cursor::new(&mut encoded);
        // Use strongest PNG compression while preserving pixels.
        let encoder = PngEncoder::new_with_quality(
            &mut cursor,
            CompressionType::Best,
            FilterType::Adaptive,
        );
        encoder
            .write_image(&rgba, width, height, ColorType::Rgba8.into())
            .map_err(|err| format!("PNG 无损压缩失败: {err}"))?;
        update_progress(app, job.id, 86.0, Some(encoded.len() as u64), None, None);
        if encoded.len() as u64 >= input_size {
            return Ok(JobResult::Skipped(
                input_size,
                format!(
                    "无损压缩未减小体积（原始: {} B, 输出: {} B）",
                    input_size,
                    encoded.len()
                ),
            ));
        }
        fs::write(output_path, &encoded).map_err(|err| format!("写入输出文件失败: {err}"))?;
        let size = encoded.len() as u64;
        update_progress(app, job.id, 95.0, Some(size), None, None);
        return Ok(JobResult::Success(size));
    } else if extension == "jpg" || extension == "jpeg" {
        // JPEG true-lossless optimization path (ImageOptim-like): optimize tables/progressive
        // without changing DCT coefficients. Requires jpegtran in runtime environment.
        update_progress(app, job.id, 64.0, None, None, None);
        let output_size = optimize_jpeg_lossless(source_path, output_path)?;
        update_progress(app, job.id, 86.0, Some(output_size), None, None);
        if output_size >= input_size {
            let _ = fs::remove_file(output_path);
            return Ok(JobResult::Skipped(
                input_size,
                format!(
                    "无损压缩未减小体积（原始: {} B, 输出: {} B）",
                    input_size, output_size
                ),
            ));
        }
        update_progress(app, job.id, 95.0, Some(output_size), None, None);
        return Ok(JobResult::Success(output_size));
    } else {
        return Err("当前仅支持 PNG/JPG/JPEG 的优化压缩".to_string());
    }
}

fn optimize_jpeg_lossless(source_path: &Path, output_path: &Path) -> Result<u64, String> {
    let status = Command::new("jpegtran")
        .arg("-copy")
        .arg("none")
        .arg("-optimize")
        .arg("-progressive")
        .arg("-outfile")
        .arg(output_path)
        .arg(source_path)
        .status();
    match status {
        Ok(exit) if exit.success() => fs::metadata(output_path)
            .map(|meta| meta.len())
            .map_err(|err| format!("读取 JPEG 输出文件元数据失败: {err}")),
        Ok(exit) => Err(format!(
            "JPEG 无损优化失败（jpegtran 退出码: {:?}）",
            exit.code()
        )),
        Err(_) => Err(
            "当前环境缺少 jpegtran，无法执行 JPEG 真无损优化；请安装 jpegtran 后重试".to_string(),
        ),
    }
}

fn compress_image_lossy(app: &AppHandle, job: &TranscodeJob) -> Result<JobResult, String> {
    let source_path = Path::new(&job.source_path);
    let output_path = Path::new(&job.output_path);
    let input_size = fs::metadata(source_path)
        .map_err(|err| format!("读取输入文件元数据失败: {err}"))?
        .len();
    let image = image::open(source_path).map_err(|err| format!("读取图片失败: {err}"))?;
    update_progress(app, job.id, 20.0, None, None, None);
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    let quality = job.quality.unwrap_or(75).clamp(1, 100);
    let target_format = job.format.as_deref().unwrap_or("jpeg");
    let tuned_quality = tuned_lossy_quality(quality, target_format);
    let encoded = if target_format == "webp" {
        let encoder = WebpEncoder::from_rgb(rgb.as_raw(), width, height);
        encoder.encode(tuned_quality as f32).to_vec()
    } else if target_format == "png" {
        let rgba = image.to_rgba8();
        let (rgba_width, rgba_height) = rgba.dimensions();
        let mut png = Vec::new();
        let mut cursor = Cursor::new(&mut png);
        let encoder = PngEncoder::new_with_quality(
            &mut cursor,
            CompressionType::Best,
            FilterType::Adaptive,
        );
        encoder
            .write_image(&rgba, rgba_width, rgba_height, ColorType::Rgba8.into())
            .map_err(|err| format!("PNG 编码失败: {err}"))?;
        png
    } else if target_format == "jpeg" {
        let mut jpeg = Vec::new();
        let mut cursor = Cursor::new(&mut jpeg);
        let encoder = JpegEncoder::new_with_quality(&mut cursor, tuned_quality);
        encoder
            .write_image(&rgb, width, height, ColorType::Rgb8.into())
            .map_err(|err| format!("JPEG 有损压缩失败: {err}"))?;
        jpeg
    } else if target_format == "gif" {
        encode_with_image_format(&image, ImageFormat::Gif).map_err(|err| format!("GIF 编码失败: {err}"))?
    } else if target_format == "bmp" {
        encode_with_image_format(&image, ImageFormat::Bmp).map_err(|err| format!("BMP 编码失败: {err}"))?
    } else {
        return Err(format!("不支持的有损输出格式: {target_format}"));
    };
    let size = encoded.len() as u64;
    update_progress(app, job.id, 86.0, Some(size), None, None);
    if size >= input_size {
        return Ok(JobResult::Skipped(
            input_size,
            format!(
                "有损压缩未减小体积（原始: {} B, 输出: {} B）",
                input_size, size
            ),
        ));
    }
    fs::write(output_path, &encoded).map_err(|err| format!("写入输出文件失败: {err}"))?;
    update_progress(app, job.id, 95.0, Some(size), None, None);
    Ok(JobResult::Success(size))
}

fn tuned_lossy_quality(input_quality: u8, target_format: &str) -> u8 {
    let quality = input_quality.clamp(1, 100);
    let tuned = match target_format {
        // Bias lower than UI quality to prioritize smaller output.
        "jpeg" => quality.saturating_sub(10),
        "webp" => quality.saturating_sub(8),
        _ => quality,
    };
    tuned.clamp(1, 100)
}

fn encode_with_image_format(image: &image::DynamicImage, format: ImageFormat) -> Result<Vec<u8>, image::ImageError> {
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, format)?;
    Ok(cursor.into_inner())
}

pub fn emit_estimate(app: &AppHandle, job_id: u64, estimated_output_size_bytes: u64) {
    #[derive(Clone, Serialize)]
    struct EstimatePayload {
        job_id: u64,
        estimated_output_size_bytes: u64,
    }
    let _ = app.emit(
        TRANSCODE_ESTIMATE_EVENT,
        EstimatePayload {
            job_id,
            estimated_output_size_bytes,
        },
    );
}
