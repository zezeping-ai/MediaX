use crate::app::media::transcode::state::{TranscodeJobStatus, TranscodeState};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

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

fn emit_progress(app: &AppHandle, payload: TranscodeProgressPayload) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = app.emit(TRANSCODE_PROGRESS_EVENT, &payload);
        let _ = app.emit(TRANSCODE_STATUS_EVENT, &payload);
    });
}

pub(super) fn update_progress(
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

pub(super) fn emit_final_progress(
    app: &AppHandle,
    job_id: u64,
    progress_percent: f64,
    status: TranscodeJobStatus,
    error_message: Option<String>,
) {
    emit_progress(
        app,
        TranscodeProgressPayload {
            job_id,
            progress_percent,
            status,
            output_size_bytes: None,
            error_message,
        },
    );
}
