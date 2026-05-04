use crate::app::media::playback::events::{
    build_media_event, MediaAudioMeterPayload, MediaMetadataPayload, MediaTelemetryPayload,
    MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_METADATA_EVENT, MEDIA_PLAYBACK_TELEMETRY_EVENT,
};
use tauri::{AppHandle, Emitter};

pub(crate) fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_TELEMETRY_EVENT,
        build_media_event("playback_telemetry", None, payload),
    );
}

pub(crate) fn emit_metadata_payloads(app: &AppHandle, payload: MediaMetadataPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_METADATA_EVENT,
        build_media_event("playback_metadata", None, payload),
    );
}

pub(crate) fn emit_audio_meter_payloads(app: &AppHandle, payload: MediaAudioMeterPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_AUDIO_METER_EVENT,
        build_media_event("playback_audio_meter", None, payload),
    );
}
