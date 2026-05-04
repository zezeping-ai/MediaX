use crate::app::media::state::MediaState;
use ffmpeg_next::{ffi, format};
use tauri::{AppHandle, Manager};

pub fn resolve_buffered_position_seconds(
    input_ctx: &format::context::Input,
    duration_seconds: f64,
    position_seconds: f64,
    is_network_source: bool,
    is_realtime_source: bool,
) -> f64 {
    let duration_seconds = duration_seconds.max(0.0);
    let position_seconds = position_seconds.max(0.0);
    if duration_seconds <= 0.0 {
        return position_seconds;
    }
    if !is_network_source {
        return duration_seconds;
    }
    if is_realtime_source {
        return position_seconds.min(duration_seconds);
    }
    let Some(buffer_ratio) = resolve_io_buffer_ratio(input_ctx) else {
        return position_seconds.min(duration_seconds);
    };
    (duration_seconds * buffer_ratio)
        .max(position_seconds)
        .min(duration_seconds)
}

pub fn update_playback_progress(
    app: &AppHandle,
    stream_generation: u32,
    position_seconds: f64,
    duration_seconds: f64,
    buffered_position_seconds: f64,
    finalize: bool,
) -> Result<(), String> {
    let state = app.state::<MediaState>();
    if !state.runtime.stream.is_generation_current(stream_generation) {
        return Ok(());
    }
    let snapshot = {
        let library = crate::app::media::state::library(&state)?.state();
        let mut playback = crate::app::media::state::playback(&state)?;
        if finalize {
            playback.stop();
            state
                .runtime
                .stream
                .set_latest_position_seconds(0.0)
                .map_err(|err| err.to_string())?;
            state
                .runtime
                .stream
                .reset_pending_seek_to_zero()
                .map_err(|err| err.to_string())?;
        } else {
            playback.sync_position(position_seconds, duration_seconds, buffered_position_seconds);
        }
        playback.snapshot(library)
    };
    crate::app::media::state::emit_playback_state_snapshot(app, snapshot, None)?;
    Ok(())
}

fn resolve_io_buffer_ratio(input_ctx: &format::context::Input) -> Option<f64> {
    // SAFETY: FFmpeg owns the format/IO context for the lifetime of `input_ctx`.
    let (offset_bytes, total_bytes) = unsafe {
        let format_ctx = input_ctx.as_ptr();
        if format_ctx.is_null() {
            return None;
        }
        let io_ctx = (*format_ctx).pb;
        if io_ctx.is_null() {
            return None;
        }
        (
            ffi::avio_seek(io_ctx, 0, ffi::SEEK_CUR),
            ffi::avio_size(io_ctx),
        )
    };
    if offset_bytes < 0 || total_bytes <= 0 {
        return None;
    }
    Some((offset_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0))
}
