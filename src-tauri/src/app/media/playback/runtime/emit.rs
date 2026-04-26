use crate::app::media::playback::events::{
    MediaAudioMeterPayload, MediaDebugPayload, MediaEventEnvelope, MediaMetadataPayload,
    MediaTelemetryPayload, MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_DEBUG_EVENT,
    MEDIA_PLAYBACK_METADATA_EVENT, MEDIA_PLAYBACK_TELEMETRY_EVENT, MEDIA_PROTOCOL_VERSION,
};
use tauri::{AppHandle, Emitter};

pub(super) fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    let at_ms = unix_epoch_ms_now();
    let message = message.into();
    let _ = app.emit(
        MEDIA_PLAYBACK_DEBUG_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_debug",
            request_id: None,
            emitted_at_ms: at_ms,
            payload: MediaDebugPayload {
                stage,
                message,
                at_ms,
            },
        },
    );
}

pub(super) fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    let emitted_at_ms = unix_epoch_ms_now();
    let _ = app.emit(
        MEDIA_PLAYBACK_TELEMETRY_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_telemetry",
            request_id: None,
            emitted_at_ms,
            payload,
        },
    );
}

pub(super) fn emit_metadata_payloads(app: &AppHandle, payload: MediaMetadataPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_METADATA_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_metadata",
            request_id: None,
            emitted_at_ms: unix_epoch_ms_now(),
            payload,
        },
    );
}

pub(crate) fn emit_audio_meter_payloads(app: &AppHandle, payload: MediaAudioMeterPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_AUDIO_METER_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_audio_meter",
            request_id: None,
            emitted_at_ms: unix_epoch_ms_now(),
            payload,
        },
    );
}

fn unix_epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
