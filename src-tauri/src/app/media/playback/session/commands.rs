use crate::app::media::error::MediaCommandError;
use crate::app::media::model::{
    CacheRecordingStatus, HardwareDecodeMode, MediaSnapshot, PlaybackQualityMode, PreviewFrame,
};
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

fn command_result<T>(
    result: crate::app::media::error::MediaResult<T>,
) -> Result<T, MediaCommandError> {
    result.map_err(Into::into)
}

#[tauri::command]
pub fn playback_get_snapshot(
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::get_snapshot(state))
}

#[tauri::command]
pub fn playback_open_source(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::open(app, state, path, request_id))
}

#[tauri::command]
pub fn playback_resume(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::play(app, state, request_id))
}

#[tauri::command]
pub fn playback_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::pause(app, state, request_id))
}

#[tauri::command]
pub fn playback_stop_session(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::stop(app, state, request_id))
}

#[tauri::command]
pub fn playback_seek_to(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    force_render: Option<bool>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::seek(
        app,
        state,
        position_seconds,
        force_render,
        request_id,
    ))
}

#[tauri::command]
pub fn playback_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_rate(app, state, playback_rate, request_id))
}

#[tauri::command]
pub fn playback_set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_volume(app, state, volume, request_id))
}

#[tauri::command]
pub fn playback_set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_muted(app, state, muted, request_id))
}

#[tauri::command]
pub fn playback_configure_decoder_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_hw_decode_mode(
        app, state, mode, request_id,
    ))
}

#[tauri::command]
pub fn playback_set_quality(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: PlaybackQualityMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_quality_mode(app, state, mode, request_id))
}

#[tauri::command]
pub fn playback_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::sync_position(
        app,
        state,
        position_seconds,
        duration_seconds,
        request_id,
    ))
}

#[tauri::command]
pub async fn playback_preview_frame(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Result<Option<PreviewFrame>, MediaCommandError> {
    command_result(
        coordinator::preview_frame(app, state, position_seconds, max_width, max_height).await,
    )
}

#[tauri::command]
pub fn playback_get_cache_recording_status(
    state: State<'_, MediaState>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::get_cache_recording_status(state))
}

#[tauri::command]
pub fn playback_start_cache_recording(
    state: State<'_, MediaState>,
    output_dir: Option<String>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::start_cache_recording(state, output_dir))
}

#[tauri::command]
pub fn playback_stop_cache_recording(
    state: State<'_, MediaState>,
) -> Result<CacheRecordingStatus, MediaCommandError> {
    command_result(coordinator::stop_cache_recording(state))
}
