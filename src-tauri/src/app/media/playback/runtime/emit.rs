use crate::app::media::playback::events::{
    build_media_event, unix_epoch_ms_now, MediaAudioMeterPayload, MediaMetadataPayload,
    MediaTelemetryPayload, MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_METADATA_EVENT,
    MEDIA_PLAYBACK_TELEMETRY_EVENT,
};
use crate::app::media::playback::debug_log::append_playback_debug_log;
use tauri::{AppHandle, Emitter};

pub(crate) fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    if !should_persist_debug_stage(stage) {
        return;
    }
    let at_ms = unix_epoch_ms_now();
    let message = message.into();
    append_playback_debug_log(app, at_ms, stage, &message);
}

pub(super) fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_TELEMETRY_EVENT,
        build_media_event("playback_telemetry", None, payload),
    );
}

pub(super) fn emit_metadata_payloads(app: &AppHandle, payload: MediaMetadataPayload) {
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

fn should_persist_debug_stage(stage: &str) -> bool {
    matches!(
        stage,
        "audio_pipeline_ready"
            | "cache_recording_error"
            | "decode_error"
            | "decode_error_detail"
            | "decoder_ready"
            | "pause_prefetch"
            | "quality_request"
            | "rate_request"
            | "restart_begin"
            | "restart_join_begin"
            | "restart_join_end"
            | "restart_stream_start"
            | "running"
            | "seek"
            | "stop"
            | "video"
    ) || stage.contains("error")
}
