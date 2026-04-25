use crate::app::media::player::coordinator;
use crate::app::media::player::state::MediaState;
use crate::app::media::types::{HardwareDecodeMode, MediaSnapshot, PlaybackQualityMode, PreviewFrame};
use tauri::{AppHandle, State};

fn command_result<T>(result: crate::app::media::error::MediaResult<T>) -> Result<T, String> {
    result.map_err(Into::into)
}

#[tauri::command]
pub fn playback_get_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    command_result(coordinator::get_snapshot(state))
}

#[tauri::command]
pub fn playback_open_source(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::open(app, state, path, request_id))
}

#[tauri::command]
pub fn playback_resume(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::play(app, state, request_id))
}

#[tauri::command]
pub fn playback_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::pause(app, state, request_id))
}

#[tauri::command]
pub fn playback_stop_session(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::stop(app, state, request_id))
}

#[tauri::command]
pub fn playback_seek_to(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    force_render: Option<bool>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
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
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::set_rate(app, state, playback_rate, request_id))
}

#[tauri::command]
pub fn playback_set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::set_volume(app, state, volume, request_id))
}

#[tauri::command]
pub fn playback_set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::set_muted(app, state, muted, request_id))
}

#[tauri::command]
pub fn playback_configure_decoder_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::set_hw_decode_mode(app, state, mode, request_id))
}

#[tauri::command]
pub fn playback_set_quality(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: PlaybackQualityMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    command_result(coordinator::set_quality_mode(app, state, mode, request_id))
}

#[tauri::command]
pub fn playback_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
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
) -> Result<Option<PreviewFrame>, String> {
    command_result(
        coordinator::preview_frame(app, state, position_seconds, max_width, max_height).await,
    )
}

// Legacy command aliases kept for compatibility with existing UI paths.
macro_rules! legacy_alias_sync {
    ($legacy_name:ident ( $($arg:ident : $arg_ty:ty),* ) => $target:ident) => {
        #[tauri::command]
        pub fn $legacy_name($($arg: $arg_ty),*) -> Result<MediaSnapshot, String> {
            $target($($arg),*)
        }
    };
}

macro_rules! legacy_alias_async_preview {
    ($legacy_name:ident => $target:ident) => {
        #[tauri::command]
        pub async fn $legacy_name(
            app: AppHandle,
            state: State<'_, MediaState>,
            position_seconds: f64,
            max_width: Option<u32>,
            max_height: Option<u32>,
        ) -> Result<Option<PreviewFrame>, String> {
            $target(app, state, position_seconds, max_width, max_height).await
        }
    };
}

legacy_alias_sync!(media_get_snapshot(state: State<'_, MediaState>) => playback_get_snapshot);
legacy_alias_sync!(media_open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>
) => playback_open_source);
legacy_alias_sync!(media_play(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>
) => playback_resume);
legacy_alias_sync!(media_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>
) => playback_pause);
legacy_alias_sync!(media_stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>
) => playback_stop_session);
legacy_alias_sync!(media_seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    force_render: Option<bool>,
    request_id: Option<String>
) => playback_seek_to);
legacy_alias_sync!(media_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>
) => playback_set_rate);
legacy_alias_sync!(media_set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>
) => playback_set_volume);
legacy_alias_sync!(media_set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>
) => playback_set_muted);
legacy_alias_sync!(media_set_hw_decode_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>
) => playback_configure_decoder_mode);
legacy_alias_sync!(media_set_quality(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: PlaybackQualityMode,
    request_id: Option<String>
) => playback_set_quality);
legacy_alias_sync!(media_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>
) => playback_sync_position);
legacy_alias_async_preview!(media_preview_frame => playback_preview_frame);
