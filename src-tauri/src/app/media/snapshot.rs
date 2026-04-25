use crate::app::media::error::MediaError;
use crate::app::media::player::events::{
    MediaEventEnvelope, MEDIA_PLAYBACK_STATE_EVENT, MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::player::state::MediaState;
use crate::app::media::types::MediaSnapshot;
use tauri::{AppHandle, Emitter, State};

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
    let envelope = MediaEventEnvelope {
        protocol_version: MEDIA_PROTOCOL_VERSION,
        event_type: "playback_state",
        request_id: request_id.clone(),
        emitted_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        payload: snapshot.clone(),
    };
    app.emit(MEDIA_PLAYBACK_STATE_EVENT, &envelope)
        .map_err(|err| format!("emit playback state failed: {err}"))?;
    Ok(snapshot)
}

pub fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?
        .state();
    let playback = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.state()
    };
    Ok(MediaSnapshot { playback, library })
}
