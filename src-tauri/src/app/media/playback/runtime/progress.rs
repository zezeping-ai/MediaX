use crate::app::media::error::MediaError;
use crate::app::media::playback::events::{
    build_media_event, MEDIA_PLAYBACK_STATE_EVENT,
};
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Emitter, Manager};

pub fn update_playback_progress(
    app: &AppHandle,
    stream_generation: u32,
    position_seconds: f64,
    duration_seconds: f64,
    finalize: bool,
) -> Result<(), String> {
    let state = app.state::<MediaState>();
    if !state.runtime.stream.is_generation_current(stream_generation) {
        return Ok(());
    }
    let snapshot = {
        let library = state
            .session
            .library
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?
            .state();
        let mut playback = state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        if finalize {
            playback.stop();
            state
                .runtime
                .stream
                .set_latest_position_seconds(0.0)
                .map_err(|err| err.to_string())?;
            state
                .runtime
                .stream
                .reset_pending_seek_to_zero()
                .map_err(|err| err.to_string())?;
        } else {
            playback.sync_position(position_seconds, duration_seconds);
        }
        playback.snapshot(library)
    };
    let envelope = build_media_event("playback_state", None, snapshot);
    app.emit(MEDIA_PLAYBACK_STATE_EVENT, &envelope)
        .map_err(|err| format!("emit playback state failed: {err}"))?;
    Ok(())
}
