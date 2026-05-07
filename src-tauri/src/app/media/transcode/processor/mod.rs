mod audio;
mod image;
mod progress;
mod video;

use super::state::{TranscodeJobKind, TranscodeJobStatus, TranscodeState};
use progress::{emit_final_progress, update_progress};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

pub use image::emit_estimate;
pub use video::probe_video_dimensions;

const TRANSCODE_DEBUG_LOG_FILE_NAME: &str = "transcode-debug.log";

enum JobResult {
    Success(u64),
    Skipped(u64, String),
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

fn run_job(app: &AppHandle, job_id: u64) {
    let job = {
        let state = app.state::<TranscodeState>();
        state.with_inner(|inner| inner.jobs.get(&job_id).cloned())
    };
    let Some(job) = job else {
        return;
    };
    append_transcode_debug_log(
        app,
        "job_start",
        &format!(
            "job_id={} kind={:?} source=\"{}\" output=\"{}\" format={:?} resolution={:?} rate={:?}",
            job.id,
            job.kind,
            job.source_path,
            job.output_path,
            job.format,
            job.resolution,
            job.playback_rate
        ),
    );
    let canceled = {
        let state = app.state::<TranscodeState>();
        state.with_inner(|inner| inner.canceled.contains(&job_id))
    };
    if canceled {
        finish_job(app, job_id, TranscodeJobStatus::Canceled, None);
        return;
    }
    let result = match job.kind {
        TranscodeJobKind::Video => {
            let rate = job.playback_rate.unwrap_or(1.0);
            let resolution = job
                .resolution
                .clone()
                .unwrap_or_else(|| "source".to_string());
            let format = job.format.clone().unwrap_or_default();
            video::transcode_video_with_progress(
                app,
                &job,
                std::path::Path::new(&job.output_path),
                &resolution,
                &format,
                rate,
            )
        }
        TranscodeJobKind::Audio => {
            let rate = job.playback_rate.unwrap_or(1.0);
            let format = job.format.clone().unwrap_or_default();
            audio::transcode_audio_with_progress(
                app,
                &job,
                std::path::Path::new(&job.output_path),
                rate,
                &format,
            )
        }
        TranscodeJobKind::ImageLossless => image::compress_image_lossless(app, &job),
        TranscodeJobKind::ImageLossy => image::compress_image_lossy(app, &job),
    };
    match result {
        Ok(JobResult::Success(output_size_bytes)) => {
            append_transcode_debug_log(
                app,
                "job_success",
                &format!("job_id={} output_size_bytes={}", job_id, output_size_bytes),
            );
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
            append_transcode_debug_log(
                app,
                "job_skipped",
                &format!(
                    "job_id={} output_size_bytes={} reason={}",
                    job_id, output_size_bytes, reason
                ),
            );
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
            append_transcode_debug_log(
                app,
                "job_failed",
                &format!("job_id={} error={}", job_id, err),
            );
            finish_job(app, job_id, TranscodeJobStatus::Failed, Some(err));
        }
    }
}

fn finish_job(
    app: &AppHandle,
    job_id: u64,
    status: TranscodeJobStatus,
    error_message: Option<String>,
) {
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
    emit_final_progress(app, job_id, final_progress_percent, status, error_message);
}

fn transcode_debug_log_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_log_dir()
        .map_err(|err| format!("resolve app log dir failed: {err}"))?;
    path.push(TRANSCODE_DEBUG_LOG_FILE_NAME);
    Ok(path)
}

fn append_transcode_debug_log(app: &AppHandle, stage: &str, message: &str) {
    let Ok(log_path) = transcode_debug_log_path(app) else {
        return;
    };
    let Some(parent_dir) = log_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent_dir).is_err() {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) else {
        return;
    };
    let _ = writeln!(file, "[{}] {}: {}", now_ms(), stage, message);
}
