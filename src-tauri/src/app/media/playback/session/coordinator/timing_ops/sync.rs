use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::session::constraints;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, State};

pub fn sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let position_seconds =
        constraints::normalize_non_negative(position_seconds, "position_seconds")?;
    let duration_seconds =
        constraints::normalize_non_negative(duration_seconds, "duration_seconds")?;
    let path = {
        let mut playback = state::playback(&state)?;
        playback.sync_position(position_seconds, duration_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state::library(&state)?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}
