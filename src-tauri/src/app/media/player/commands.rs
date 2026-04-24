use crate::app::media::player::coordinator;
use crate::app::media::player::state::MediaState;
use crate::app::media::types::{HardwareDecodeMode, MediaSnapshot, PreviewFrame};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn media_get_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    coordinator::get_snapshot(state)
}

#[tauri::command]
pub fn media_open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::open(app, state, path, request_id)
}

#[tauri::command]
pub fn media_play(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::play(app, state, request_id)
}

#[tauri::command]
pub fn media_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::pause(app, state, request_id)
}

#[tauri::command]
pub fn media_stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::stop(app, state, request_id)
}

#[tauri::command]
pub fn media_seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    force_render: Option<bool>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::seek(app, state, position_seconds, force_render, request_id)
}

#[tauri::command]
pub fn media_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::set_rate(app, state, playback_rate, request_id)
}

#[tauri::command]
pub fn media_set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::set_volume(app, state, volume, request_id)
}

#[tauri::command]
pub fn media_set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::set_muted(app, state, muted, request_id)
}

#[tauri::command]
pub fn media_set_hw_decode_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::set_hw_decode_mode(app, state, mode, request_id)
}

#[tauri::command]
pub fn media_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    coordinator::sync_position(app, state, position_seconds, duration_seconds, request_id)
}

#[tauri::command]
pub async fn media_preview_frame(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Result<Option<PreviewFrame>, String> {
    coordinator::preview_frame(app, state, position_seconds, max_width, max_height).await
}
