use crate::app::media::playback::events::{
    build_media_event, MediaErrorPayload, MEDIA_PLAYBACK_ERROR_EVENT,
};
use tauri::{AppHandle, Emitter};

pub(super) fn emit_error_events(app: &AppHandle, code: &'static str, message: String) {
    let error_payload = MediaErrorPayload { code, message };
    let _ = app.emit(
        MEDIA_PLAYBACK_ERROR_EVENT,
        build_media_event("playback_error", None, error_payload),
    );
}
