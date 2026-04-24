use crate::app::media::player::events::{
    MediaEventEnvelope, MEDIA_PROTOCOL_VERSION, MEDIA_STATE_EVENT, MEDIA_STATE_EVENT_V2,
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
    app.emit(MEDIA_STATE_EVENT, &snapshot)
        .map_err(|err| format!("emit media state failed: {err}"))?;
    let envelope = MediaEventEnvelope {
        protocol_version: MEDIA_PROTOCOL_VERSION,
        event_type: "state",
        request_id,
        emitted_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        payload: snapshot.clone(),
    };
    app.emit(MEDIA_STATE_EVENT_V2, &envelope)
        .map_err(|err| format!("emit media state v2 failed: {err}"))?;
    Ok(snapshot)
}

pub fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .library
        .lock()
        .map_err(|_| "media library state poisoned".to_string())?
        .state();
    let playback = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.state()
    };
    Ok(MediaSnapshot { playback, library })
}
