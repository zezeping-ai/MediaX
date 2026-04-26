use super::command_result;
use crate::app::media::error::MediaCommandError;
use crate::app::media::model::PreviewFrame;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

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
