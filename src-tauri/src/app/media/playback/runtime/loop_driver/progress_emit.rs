use super::DecodeRuntime;
use crate::app::media::playback::runtime::progress::{
    resolve_buffered_position_seconds, update_playback_progress,
};
use crate::app::media::playback::runtime::write_latest_stream_position;
use crate::app::media::state::MediaState;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(super) fn write_position_and_maybe_emit_progress(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    position_seconds: f64,
    finished: bool,
    emit_interval: Duration,
    persist_latest_position: bool,
) -> Result<bool, String> {
    let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
    let mut normalized_position = position_seconds.max(0.0);
    if duration_seconds > 0.0 {
        normalized_position = normalized_position.min(duration_seconds);
    }
    runtime.loop_state.progress_position_seconds = normalized_position;
    if persist_latest_position {
        write_latest_stream_position(&app.state::<MediaState>(), normalized_position)?;
    }
    if runtime.loop_state.last_progress_emit.elapsed() < emit_interval {
        return Ok(false);
    }
    let buffered_position_seconds = resolve_buffered_position_seconds(
        &runtime.video_ctx.input_ctx,
        duration_seconds,
        normalized_position,
        runtime.is_network_source,
        runtime.is_realtime_source,
    );
    update_playback_progress(
        app,
        stream_generation,
        normalized_position,
        duration_seconds,
        buffered_position_seconds,
        finished,
    )?;
    runtime.loop_state.last_progress_emit = Instant::now();
    Ok(true)
}

