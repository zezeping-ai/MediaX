use super::emit::emit_debug;
use super::progress::{resolve_buffered_position_seconds, update_playback_progress};
use super::seek_control;
use super::video_pipeline;
use super::{DecodeRuntime, AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT};
use crate::app::media::playback::runtime::audio::effective_playback_rate;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::{MediaState, TimingControls};
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

mod pacing;
mod packet_flow;
mod tail;

use self::pacing::{
    refresh_audio_rate, refresh_tail_audio_rate, should_wait_for_audio_queue_drain,
    should_wait_for_rate_switch_drain,
    should_wait_for_decode_lead,
};
use self::packet_flow::{drain_video_frames, handle_packet};
pub(super) use self::tail::finish_decode_runtime;

pub(super) fn run_decode_loop(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    loop {
        if should_exit_loop(app, stop_flag, stream_generation) {
            return Ok(());
        }
        if drive_pause_prefetch(app, renderer, runtime, stream_generation)? {
            continue;
        }
        refresh_audio_rate(app, runtime, timing_controls);
        if should_wait_for_rate_switch_drain(app, runtime) {
            std::thread::sleep(Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS));
            continue;
        }
        if should_wait_for_decode_lead(runtime) {
            std::thread::sleep(Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS));
            continue;
        }
        if should_wait_for_audio_queue_drain(app, runtime) {
            std::thread::sleep(Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS));
            continue;
        }
        if apply_pending_seek(app, renderer, timing_controls, runtime)? {
            continue;
        }
        let mut packet = Packet::empty();
        match packet.read(&mut runtime.video_ctx.input_ctx) {
            Ok(_) => handle_packet(
                app,
                renderer,
                source,
                stop_flag,
                timing_controls,
                runtime,
                stream_generation,
                packet,
            )?,
            Err(err) => {
                if should_break_after_read_error(stop_flag, runtime, &err) {
                    break;
                }
                if should_retry_after_read_error(runtime, &err) {
                    sleep_for_read_retry(&err);
                }
            }
        }
    }
    Ok(())
}

fn drive_pause_prefetch(
    app: &AppHandle,
    renderer: &RendererState,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<bool, String> {
    if !app
        .state::<MediaState>()
        .runtime
        .pause_prefetch_active
        .load(Ordering::Relaxed)
    {
        if runtime.loop_state.pause_prefetch_mode {
            exit_pause_prefetch(runtime);
        }
        return Ok(false);
    }
    enter_pause_prefetch(renderer, runtime);
    let mut packet = Packet::empty();
    match packet.read(&mut runtime.video_ctx.input_ctx) {
        Ok(_) => {
            packet_flow::track_packet_windows(runtime, &packet);
            emit_pause_prefetch_progress(app, runtime, stream_generation)?;
        }
        Err(err) => {
            emit_pause_prefetch_progress(app, runtime, stream_generation)?;
            sleep_for_read_retry(&err);
        }
    }
    Ok(true)
}

fn should_break_after_read_error(
    stop_flag: &Arc<AtomicBool>,
    runtime: &DecodeRuntime,
    err: &FfmpegError,
) -> bool {
    match err {
        FfmpegError::Eof => stop_flag.load(Ordering::Relaxed) || !runtime.should_tail_eof,
        _ => false,
    }
}

fn should_retry_after_read_error(runtime: &DecodeRuntime, err: &FfmpegError) -> bool {
    match err {
        FfmpegError::Eof => runtime.should_tail_eof,
        _ => runtime.should_tail_eof,
    }
}

fn sleep_for_read_retry(err: &FfmpegError) {
    let delay_ms = match err {
        FfmpegError::Eof => 200,
        _ => 50,
    };
    std::thread::sleep(Duration::from_millis(delay_ms));
}

fn enter_pause_prefetch(renderer: &RendererState, runtime: &mut DecodeRuntime) {
    if runtime.loop_state.pause_prefetch_mode {
        return;
    }
    if let Some(audio) = runtime.audio_pipeline.as_ref() {
        audio.output.pause_and_clear_queue();
    }
    renderer.reset_timeline(runtime.loop_state.current_position_seconds.max(0.0), 1.0);
    runtime.loop_state.pause_prefetch_mode = true;
    runtime.loop_state.pause_prefetch_logged_buffered_seconds = None;
}

fn exit_pause_prefetch(runtime: &mut DecodeRuntime) {
    if !runtime.loop_state.pause_prefetch_mode {
        return;
    }
    if let Some(audio) = runtime.audio_pipeline.as_ref() {
        audio.output.resume();
    }
    runtime.loop_state.pause_prefetch_mode = false;
    runtime.loop_state.pause_prefetch_logged_buffered_seconds = None;
}

fn emit_pause_prefetch_progress(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    if runtime.loop_state.last_progress_emit.elapsed() < Duration::from_millis(200) {
        return Ok(());
    }
    let position_seconds = app
        .state::<MediaState>()
        .runtime
        .stream
        .latest_position_seconds()
        .map_err(|err| err.to_string())?;
    runtime.loop_state.current_position_seconds = position_seconds.max(0.0);
    let buffered_position_seconds = resolve_buffered_position_seconds(
        &runtime.video_ctx.input_ctx,
        runtime.video_ctx.duration_seconds,
        runtime.loop_state.current_position_seconds,
        runtime.is_network_source,
        runtime.is_realtime_source,
    );
    let should_log_progress = runtime
        .loop_state
        .pause_prefetch_logged_buffered_seconds
        .map(|previous| buffered_position_seconds >= previous + 0.5)
        .unwrap_or(true);
    if should_log_progress {
        emit_debug(
            app,
            "pause_prefetch_progress",
            format!(
                "position={:.3}s buffered={:.3}s duration={:.3}s",
                runtime.loop_state.current_position_seconds,
                buffered_position_seconds,
                runtime.video_ctx.duration_seconds.max(0.0),
            ),
        );
        runtime.loop_state.pause_prefetch_logged_buffered_seconds = Some(buffered_position_seconds);
    }
    update_playback_progress(
        app,
        stream_generation,
        runtime.loop_state.current_position_seconds,
        runtime.video_ctx.duration_seconds,
        buffered_position_seconds,
        false,
    )?;
    runtime.loop_state.last_progress_emit = Instant::now();
    Ok(())
}

fn should_exit_loop(app: &AppHandle, stop_flag: &Arc<AtomicBool>, stream_generation: u32) -> bool {
    if !app
        .state::<MediaState>()
        .runtime
        .stream
        .is_generation_current(stream_generation)
    {
        emit_debug(
            app,
            "stop",
            "stale decode generation observed; exiting decode loop",
        );
        return true;
    }
    if stop_flag.load(Ordering::Relaxed) {
        emit_debug(app, "stop", "stop flag observed; exiting decode loop");
        return true;
    }
    false
}

fn apply_pending_seek(
    app: &AppHandle,
    renderer: &RendererState,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
) -> Result<bool, String> {
    let Some(target_seconds) = seek_control::take_pending_seek_seconds(app)? else {
        return Ok(false);
    };
    emit_debug(app, "seek", format!("apply seek to {target_seconds:.3}s"));
    seek_control::apply_seek_to_stream(
        &mut runtime.video_ctx.input_ctx,
        runtime.video_ctx.decoder.as_mut(),
        target_seconds,
        &mut runtime.loop_state.playback_clock,
        &mut runtime.loop_state.current_position_seconds,
        runtime.audio_pipeline.as_mut(),
    )?;
    renderer.reset_timeline(
        target_seconds.max(0.0),
        effective_playback_rate(
            timing_controls.playback_rate_value(),
            runtime.is_realtime_source,
        )
        .as_f64(),
    );
    runtime.loop_state.reset_audio_sync_state();
    runtime.loop_state.active_seek_target_seconds = Some(target_seconds.max(0.0));
    runtime.loop_state.begin_seek_refill(Duration::from_millis(220));
    runtime.loop_state.begin_seek_settle(Duration::from_millis(700));
    if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
        audio_state.stats.seek_refill_logged = false;
    }
    runtime.loop_state.last_video_pts_seconds = None;
    runtime.loop_state.last_progress_emit = Instant::now() - Duration::from_millis(250);
    Ok(true)
}
