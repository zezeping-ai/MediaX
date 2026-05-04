use super::command_result;
use crate::app::media::error::MediaCommandError;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use serde::Deserialize;
use tauri::{AppHandle, State};

#[cfg(desktop)]
use crate::app::shell::open_request::schedule_native_open_local_dialog;

#[derive(Deserialize)]
pub struct PlaybackOpenSourceArgs {
    #[serde(alias = "path")]
    pub source: String,
    pub request_id: Option<String>,
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
    args: PlaybackOpenSourceArgs,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::open(app, state, args.source, args.request_id))
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

/// Returns true when the desktop native picker was scheduled (frontend must not call open+play again).
#[tauri::command]
pub fn playback_pick_local_file(app: AppHandle) -> Result<bool, MediaCommandError> {
    #[cfg(desktop)]
    {
        schedule_native_open_local_dialog(&app);
        Ok(true)
    }
    #[cfg(not(desktop))]
    {
        Ok(false)
    }
}
