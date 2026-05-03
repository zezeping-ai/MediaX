use super::command_result;
use crate::app::media::error::MediaCommandError;
use crate::app::media::model::CacheRecordingStatus;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn playback_get_cache_recording_status(
    state: State<'_, MediaState>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::get_cache_recording_status(state))
}

#[tauri::command]
pub fn playback_start_cache_recording(
    app: AppHandle,
    state: State<'_, MediaState>,
    output_dir: Option<String>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::start_cache_recording(app, state, output_dir))
}

#[tauri::command]
pub fn playback_stop_cache_recording(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::stop_cache_recording(app, state))
}
