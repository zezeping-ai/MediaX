use super::MediaState;
use crate::app::media::error::MediaError;
use crate::app::media::library::MediaLibraryService;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::events::{
    build_media_event, MEDIA_PLAYBACK_STATE_EVENT,
};
use crate::app::media::playback::session::service::MediaPlaybackService;
use std::sync::MutexGuard;
use tauri::{AppHandle, Emitter, State};

pub fn playback<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaPlaybackService>, String> {
    state
        .session
        .playback
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())
}

pub fn library<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaLibraryService>, String> {
    state
        .session
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())
}

pub fn emit_snapshot(
    app: &AppHandle,
    state: &State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    emit_snapshot_with_request_id(app, state, None)
}

pub fn emit_snapshot_with_request_id(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    let snapshot = snapshot_from_state(state)?;
    emit_playback_state_snapshot(app, snapshot.clone(), request_id)?;
    Ok(snapshot)
}

pub fn emit_playback_state_snapshot(
    app: &AppHandle,
    snapshot: MediaSnapshot,
    request_id: Option<String>,
) -> Result<(), String> {
    let envelope = build_media_event("playback_state", request_id, snapshot);
    app.emit(MEDIA_PLAYBACK_STATE_EVENT, &envelope)
        .map_err(|err| format!("emit playback state failed: {err}"))?;
    Ok(())
}

pub fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .session
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?
        .state();
    let playback = {
        let playback = state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.snapshot(library.clone())
    };
    Ok(playback)
}
