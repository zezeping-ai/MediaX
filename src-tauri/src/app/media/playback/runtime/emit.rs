use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::events::{
    build_media_event, unix_epoch_ms_now, MediaAudioMeterPayload, MediaMetadataPayload,
    MediaTelemetryPayload, MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_METADATA_EVENT,
    MEDIA_PLAYBACK_TELEMETRY_EVENT,
};
use tauri::{AppHandle, Emitter};

const RELEASE_DEBUG_MESSAGE_MAX_CHARS: usize = 200;

pub(crate) fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    if !should_persist_debug_stage(stage) {
        return;
    }
    let at_ms = unix_epoch_ms_now();
    let message = normalize_debug_message_for_build(stage, message.into());
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
    if cfg!(debug_assertions) {
        // Dev build: keep full-fidelity debug logs for troubleshooting.
        return true;
    }
    should_persist_release_debug_stage(stage)
}

fn should_persist_release_debug_stage(stage: &str) -> bool {
    matches!(
        stage,
        "audio_output_underrun"
            | "audio_queue_low"
            | "audio_queue_recovered"
            | "audio_decode_supply_gap"
            | "audio_decode_supply_recovered"
            | "decode_error"
            | "decode_error_detail"
            | "seek"
            | "stop"
    ) || stage.contains("error")
}

fn normalize_debug_message_for_build(stage: &str, message: String) -> String {
    if cfg!(debug_assertions) || stage.contains("error") {
        return message;
    }
    truncate_chars(message, RELEASE_DEBUG_MESSAGE_MAX_CHARS)
}

fn truncate_chars(input: String, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_none() {
        return truncated;
    }
    format!("{truncated}...")
}
