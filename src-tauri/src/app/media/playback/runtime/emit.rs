use crate::app::media::playback::events::{
    build_media_event, unix_epoch_ms_now, MediaAudioMeterPayload, MediaMetadataPayload,
    MediaTelemetryPayload, MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_METADATA_EVENT,
    MEDIA_PLAYBACK_TELEMETRY_EVENT,
};
use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Emitter, Manager};

pub(crate) fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    let at_ms = unix_epoch_ms_now();
    let message = message.into();
    if app
        .state::<MediaState>()
        .controls
        .debug
        .playback_log_enabled()
    {
        append_playback_debug_log(app, at_ms, stage, &message);
    }
    // Keep debug diagnostics off the frontend event bus. High-frequency debug pushes add
    // avoidable UI-thread/event-loop pressure and do not belong on the critical playback path.
    let _ = (stage, message.as_str(), at_ms);
}

pub(super) fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    if !app
        .state::<MediaState>()
        .controls
        .debug
        .frontend_diagnostics_enabled()
    {
        return;
    }
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
    if !app
        .state::<MediaState>()
        .controls
        .debug
        .frontend_diagnostics_enabled()
    {
        return;
    }
    let _ = app.emit(
        MEDIA_PLAYBACK_AUDIO_METER_EVENT,
        build_media_event("playback_audio_meter", None, payload),
    );
}
