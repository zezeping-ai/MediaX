use super::command_result;
use crate::app::media::error::MediaError;
use crate::app::media::error::MediaCommandError;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::{
    HardwareDecodeMode, PlaybackChannelRouting, PlaybackQualityMode,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

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
#[allow(non_snake_case)]
pub fn playback_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: Option<f64>,
    playbackRate: Option<f64>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    let playback_rate = playback_rate
        .or(playbackRate)
        .ok_or_else(|| MediaError::invalid_input("missing playback_rate"))?;
    emit_debug(&app, "rate_request", format!("requested playback_rate={playback_rate:.3}"));
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
pub fn playback_set_left_channel_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_left_channel_volume(
        app, state, volume, request_id,
    ))
}

#[tauri::command]
pub fn playback_set_right_channel_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_right_channel_volume(
        app, state, volume, request_id,
    ))
}

#[tauri::command]
pub fn playback_set_left_channel_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_left_channel_muted(
        app, state, muted, request_id,
    ))
}

#[tauri::command]
pub fn playback_set_right_channel_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_right_channel_muted(
        app, state, muted, request_id,
    ))
}

#[tauri::command]
pub fn playback_set_channel_routing(
    app: AppHandle,
    state: State<'_, MediaState>,
    routing: PlaybackChannelRouting,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::set_channel_routing(
        app, state, routing, request_id,
    ))
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
    emit_debug(&app, "quality_request", format!("requested quality_mode={mode:?}"));
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
