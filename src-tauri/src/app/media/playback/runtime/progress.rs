use crate::app::media::error::MediaError;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::events::{
    MediaEventEnvelope, MEDIA_PLAYBACK_STATE_EVENT, MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::state::MediaState;
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
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?
            .state();
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        if finalize {
            playback.stop();
            state
                .stream
                .set_latest_position_seconds(0.0)
                .map_err(|err| err.to_string())?;
            state
                .stream
                .reset_pending_seek_to_zero()
                .map_err(|err| err.to_string())?;
        } else {
            playback.sync_position(position_seconds, duration_seconds);
        }
        let playback_state = playback.state();
        MediaSnapshot {
            playback: playback_state,
            library,
        }
    };
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
    Ok(())
}
