use crate::app::media::playback::events::{
    MediaErrorPayload, MediaEventEnvelope, MEDIA_PLAYBACK_ERROR_EVENT, MEDIA_PROTOCOL_VERSION,
};
use tauri::{AppHandle, Emitter};

pub(super) fn emit_error_events(app: &AppHandle, code: &'static str, message: String) {
    let emitted_at_ms = unix_epoch_ms_now();
    let error_payload = MediaErrorPayload { code, message };
    let _ = app.emit(
        MEDIA_PLAYBACK_ERROR_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_error",
            request_id: None,
            emitted_at_ms,
            payload: error_payload,
        },
    );
}

fn unix_epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
