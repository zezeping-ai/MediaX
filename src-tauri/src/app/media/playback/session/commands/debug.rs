use super::command_result;
use crate::app::media::error::MediaCommandError;
use crate::app::media::playback::debug_log;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn playback_get_debug_log_path(app: AppHandle) -> Result<String, MediaCommandError> {
    command_result(
        debug_log::playback_debug_log_path(&app).map(|path| path.display().to_string()),
    )
}

#[tauri::command]
pub fn playback_clear_debug_log(app: AppHandle) -> Result<String, MediaCommandError> {
    command_result(debug_log::clear_playback_debug_log(&app))
}

#[tauri::command]
pub fn playback_set_debug_log_enabled(
    state: State<'_, MediaState>,
    enabled: bool,
) -> Result<bool, MediaCommandError> {
    state.debug_controls.set_playback_log_enabled(enabled);
    Ok(enabled)
}
