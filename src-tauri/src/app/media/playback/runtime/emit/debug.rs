use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::events::unix_epoch_ms_now;
use tauri::AppHandle;

pub(crate) fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    if !should_persist_debug_stage(stage) {
        return;
    }
    let at_ms = unix_epoch_ms_now();
    let message = message.into();
    append_playback_debug_log(app, at_ms, stage, &message);
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
            | "restart_join_timeout"
            | "restart_skipped"
            | "restart_stream_start"
            | "running"
            | "seek"
            | "stop"
            | "video"
    ) || stage.contains("error")
}
