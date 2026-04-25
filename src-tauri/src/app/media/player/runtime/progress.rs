use crate::app::media::player::events::{
    MediaEventEnvelope, MEDIA_PLAYBACK_STATE_EVENT, MEDIA_PROTOCOL_VERSION, MEDIA_STATE_EVENT,
    MEDIA_STATE_EVENT_V2,
};
use crate::app::media::player::state::MediaState;
use crate::app::media::types::MediaSnapshot;
use tauri::{AppHandle, Emitter, Manager};

pub fn update_playback_progress(
    app: &AppHandle,
    position_seconds: f64,
    duration_seconds: f64,
    finalize: bool,
) -> Result<(), String> {
    let state = app.state::<MediaState>();
    let snapshot = {
        let library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?
            .state();
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        if finalize {
            playback.stop();
            let mut latest_position = state
                .latest_stream_position_seconds
                .lock()
                .map_err(|_| "latest position state poisoned".to_string())?;
            *latest_position = 0.0;
            let mut pending_seek = state
                .pending_seek_seconds
                .lock()
                .map_err(|_| "pending seek state poisoned".to_string())?;
            *pending_seek = Some(0.0);
        } else {
            playback.sync_position(position_seconds, duration_seconds);
        }
        let playback_state = playback.state();
        MediaSnapshot {
            playback: playback_state,
            library,
        }
    };
    app.emit(MEDIA_STATE_EVENT, &snapshot)
        .map_err(|err| format!("emit media state failed: {err}"))?;
    let envelope = MediaEventEnvelope {
        protocol_version: MEDIA_PROTOCOL_VERSION,
        event_type: "playback_state",
        request_id: None,
        emitted_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        payload: snapshot,
    };
    app.emit(MEDIA_PLAYBACK_STATE_EVENT, &envelope)
        .map_err(|err| format!("emit playback state failed: {err}"))?;
    let legacy_v2_envelope = MediaEventEnvelope {
        event_type: "state",
        ..envelope
    };
    app.emit(MEDIA_STATE_EVENT_V2, &legacy_v2_envelope)
    .map_err(|err| format!("emit media state v2 failed: {err}"))?;
    Ok(())
}
