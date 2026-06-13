use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::events::{
    build_media_event, unix_epoch_ms_now, MediaAudioMeterPayload, MediaMetadataPayload,
    MediaTelemetryPayload, MEDIA_PLAYBACK_AUDIO_METER_EVENT, MEDIA_PLAYBACK_METADATA_EVENT,
    MEDIA_PLAYBACK_TELEMETRY_EVENT,
};
use tauri::{AppHandle, Emitter};

pub(crate) fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    append_sync_telemetry_log(app, &payload);
    let _ = app.emit(
        MEDIA_PLAYBACK_TELEMETRY_EVENT,
        build_media_event("playback_telemetry", None, payload),
    );
}

fn append_sync_telemetry_log(app: &AppHandle, payload: &MediaTelemetryPayload) {
    let av_offset_ms = match (
        payload.current_presented_video_pts_seconds,
        payload.current_audio_clock_seconds,
    ) {
        (Some(presented), Some(audio)) if presented.is_finite() && audio.is_finite() => {
            Some((presented - audio) * 1000.0)
        }
        _ => payload
            .audio_drift_seconds
            .filter(|value| value.is_finite())
            .map(|value| value * 1000.0),
    };
    let decode_ahead_ms = match (
        payload.current_submitted_video_pts_seconds,
        payload.current_audio_clock_seconds,
    ) {
        (Some(submitted), Some(audio)) if submitted.is_finite() && audio.is_finite() => {
            Some((submitted - audio) * 1000.0)
        }
        _ => None,
    };
    let present_lag_ms = payload
        .render_present_lag_ms
        .filter(|value| value.is_finite());
    let submit_lead_ms = payload
        .video_submit_lead_ms
        .filter(|value| value.is_finite());
    let gap_ms = payload
        .video_pts_gap_seconds
        .filter(|value| value.is_finite())
        .map(|value| value * 1000.0);
    let jitter_max_ms = payload
        .video_timestamps
        .as_ref()
        .map(|stats| stats.jitter_max_ms)
        .filter(|value| value.is_finite());
    let jitter_avg_ms = payload
        .video_timestamps
        .as_ref()
        .map(|stats| stats.jitter_avg_ms)
        .filter(|value| value.is_finite());
    let message = format!(
        "clock={:.3}s audio={} video_pts={} presented={} av_offset_ms={} decode_ahead_ms={} present_lag_ms={} submit_lead_ms={} gap_ms={} jitter_avg_ms={} jitter_max_ms={} aq={} vq={} render_fps={:.1}",
        payload.clock_seconds,
        fmt_optional_seconds(payload.current_audio_clock_seconds),
        fmt_optional_seconds(payload.current_video_pts_seconds),
        fmt_optional_seconds(payload.current_presented_video_pts_seconds),
        fmt_optional_ms(av_offset_ms),
        fmt_optional_ms(decode_ahead_ms),
        fmt_optional_ms(present_lag_ms),
        fmt_optional_ms(submit_lead_ms),
        fmt_optional_ms(gap_ms),
        fmt_optional_ms(jitter_avg_ms),
        fmt_optional_ms(jitter_max_ms),
        fmt_optional_usize(payload.audio_queue_depth_sources),
        payload.queue_depth,
        payload.render_fps,
    );
    append_playback_debug_log(app, unix_epoch_ms_now(), "sync_telemetry", &message);
}

fn fmt_optional_seconds(value: Option<f64>) -> String {
    value
        .filter(|seconds| seconds.is_finite())
        .map(|seconds| format!("{seconds:.3}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_optional_ms(value: Option<f64>) -> String {
    value
        .map(|ms| format!("{ms:.1}"))
        .unwrap_or_else(|| "-".to_string())
}

fn fmt_optional_usize(value: Option<usize>) -> String {
    value
        .map(|depth| depth.to_string())
        .unwrap_or_else(|| "-".to_string())
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
