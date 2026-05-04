use super::emit::emit_debug;
use super::progress::resolve_buffered_position_seconds;
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
mod progress_emit;
mod tail;

const SEEK_REFILL_WINDOW_MS_DEFAULT: u64 = 220;
const SEEK_SETTLE_WINDOW_MS_DEFAULT: u64 = 700;
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(200);

use self::pacing::{
    refresh_audio_rate, refresh_tail_audio_rate, should_wait_for_audio_queue_drain,
    should_prioritize_audio_continuity, should_wait_for_rate_switch_drain,
    should_wait_for_decode_lead,
};
use self::packet_flow::{drain_video_frames, handle_packet};
use self::progress_emit::write_position_and_maybe_emit_progress;
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
            sleep_with_stop_flag(
                stop_flag,
                Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS),
            );
            continue;
        }
        if should_wait_for_audio_queue_drain(app, runtime) {
            sleep_with_stop_flag(
                stop_flag,
                Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS),
            );
            continue;
        }
        if should_prioritize_audio_continuity(runtime) && should_wait_for_decode_lead(runtime) {
            sleep_with_stop_flag(
                stop_flag,
                Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS),
            );
            continue;
        }
        if should_wait_for_decode_lead(runtime) {
            sleep_with_stop_flag(
                stop_flag,
                Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS),
            );
            continue;
        }
        if apply_pending_seek(app, renderer, timing_controls, runtime)? {
            continue;
        }
        let mut packet = Packet::empty();
        match if let Some(stashed) = runtime.demux_packet_stash.take() {
            packet = stashed;
            Ok(())
        } else {
            packet.read(&mut runtime.video_ctx.input_ctx)
        } {
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
                match read_error_strategy(stop_flag, runtime, &err) {
                    ReadErrorStrategy::Break => break,
                    ReadErrorStrategy::Retry(delay) => sleep_with_stop_flag(stop_flag, delay),
                    ReadErrorStrategy::Ignore => {}
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
    let pause_prefetch_active = app
        .state::<MediaState>()
        .runtime
        .pause_prefetch_active
        .load(Ordering::Relaxed);
    if !pause_prefetch_active {
        update_pause_prefetch_mode(false, renderer, runtime);
        return Ok(false);
    }

    update_pause_prefetch_mode(true, renderer, runtime);
    let mut packet = Packet::empty();
    match packet.read(&mut runtime.video_ctx.input_ctx) {
        Ok(_) => handle_pause_prefetch_read_ok(app, runtime, stream_generation, &packet)?,
        Err(err) => handle_pause_prefetch_read_err(app, runtime, stream_generation, &err)?,
    }
    Ok(true)
}

enum ReadErrorStrategy {
    Break,
    Retry(Duration),
    Ignore,
}

fn read_error_strategy(
    stop_flag: &Arc<AtomicBool>,
    runtime: &DecodeRuntime,
    err: &FfmpegError,
) -> ReadErrorStrategy {
    match err {
        FfmpegError::Eof if stop_flag.load(Ordering::Relaxed) || !runtime.should_tail_eof => {
            ReadErrorStrategy::Break
        }
        FfmpegError::Eof if runtime.should_tail_eof => {
            ReadErrorStrategy::Retry(Duration::from_millis(200))
        }
        _ if runtime.should_tail_eof => ReadErrorStrategy::Retry(Duration::from_millis(50)),
        _ => ReadErrorStrategy::Ignore,
    }
}

fn sleep_for_read_retry(err: &FfmpegError) {
    let delay = match err {
        FfmpegError::Eof => Duration::from_millis(200),
        _ => Duration::from_millis(50),
    };
    std::thread::sleep(delay);
}

fn update_pause_prefetch_mode(
    should_enable: bool,
    renderer: &RendererState,
    runtime: &mut DecodeRuntime,
) {
    if should_enable == runtime.loop_state.pause_prefetch_mode {
        return;
    }
    if should_enable {
        if let Some(audio) = runtime.audio_pipeline.as_ref() {
            audio.output.pause_and_clear_queue();
        }
        renderer.reset_timeline(runtime.loop_state.progress_position_seconds.max(0.0), 1.0);
    } else if let Some(audio) = runtime.audio_pipeline.as_ref() {
        audio.output.resume();
    }
    runtime.loop_state.pause_prefetch_mode = should_enable;
    runtime.loop_state.pause_prefetch_logged_buffered_seconds = None;
}

fn handle_pause_prefetch_read_ok(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    packet_flow::track_packet_windows(runtime, packet);
    emit_pause_prefetch_progress(app, runtime, stream_generation)
}

fn handle_pause_prefetch_read_err(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    err: &FfmpegError,
) -> Result<(), String> {
    emit_pause_prefetch_progress(app, runtime, stream_generation)?;
    sleep_for_read_retry(err);
    Ok(())
}

pub(super) fn sleep_with_stop_flag(stop_flag: &AtomicBool, total: Duration) {
    const SLICE_MS: u64 = 2;
    let mut remaining = total;
    while !remaining.is_zero() {
        if stop_flag.load(Ordering::Relaxed) {
            return;
        }
        let step = remaining.min(Duration::from_millis(SLICE_MS));
        std::thread::sleep(step);
        remaining = remaining.saturating_sub(step);
    }
}

fn emit_pause_prefetch_progress(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    if runtime.loop_state.last_progress_emit.elapsed() < PROGRESS_EMIT_INTERVAL {
        return Ok(());
    }
    let position_seconds = app
        .state::<MediaState>()
        .runtime
        .stream
        .latest_position_seconds()
        .map_err(|err| err.to_string())?;
    runtime.loop_state.progress_position_seconds = position_seconds.max(0.0);
    let buffered_position_seconds = resolve_buffered_position_seconds(
        &runtime.video_ctx.input_ctx,
        runtime.video_ctx.duration_seconds,
        runtime.loop_state.progress_position_seconds,
        runtime.is_network_source,
        runtime.is_realtime_source,
    );
    let should_log_progress = should_log_pause_prefetch_progress(runtime, buffered_position_seconds);
    if should_log_progress {
        emit_debug(
            app,
            "pause_prefetch_progress",
            format!(
                "position={:.3}s buffered={:.3}s duration={:.3}s",
                runtime.loop_state.progress_position_seconds,
                buffered_position_seconds,
                runtime.video_ctx.duration_seconds.max(0.0),
            ),
        );
        runtime.loop_state.pause_prefetch_logged_buffered_seconds = Some(buffered_position_seconds);
    }
    let _ = write_position_and_maybe_emit_progress(
        app,
        runtime,
        stream_generation,
        runtime.loop_state.progress_position_seconds,
        false,
        PROGRESS_EMIT_INTERVAL,
        false,
    )?;
    Ok(())
}

fn should_log_pause_prefetch_progress(runtime: &DecodeRuntime, buffered_position_seconds: f64) -> bool {
    runtime
        .loop_state
        .pause_prefetch_logged_buffered_seconds
        .map(|previous| buffered_position_seconds >= previous + 0.5)
        .unwrap_or(true)
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
    if target_seconds <= f64::EPSILON
        && runtime.loop_state.progress_position_seconds <= f64::EPSILON
        && runtime.loop_state.last_video_pts_seconds.is_none()
    {
        emit_debug(app, "seek", "skip startup no-op seek to 0.000s");
        return Ok(false);
    }
    emit_debug(app, "seek", format!("apply seek to {target_seconds:.3}s"));
    reset_runtime_after_seek(runtime);
    seek_control::apply_seek_to_stream(
        &mut runtime.video_ctx.input_ctx,
        runtime.video_ctx.decoder.as_mut(),
        target_seconds,
        &mut runtime.loop_state.playback_clock,
        &mut runtime.loop_state.progress_position_seconds,
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
    let (seek_refill_window, seek_settle_window) = seek_windows_for_profile(runtime);
    runtime
        .loop_state
        .begin_seek_refill(seek_refill_window);
    runtime
        .loop_state
        .begin_seek_settle(seek_settle_window);
    runtime
        .loop_state
        .begin_audio_sync_warmup(Duration::from_millis(2500));
    if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
        audio_state.stats.seek_refill_logged = false;
    }
    runtime.loop_state.last_video_pts_seconds = None;
    runtime.loop_state.last_progress_emit = Instant::now() - Duration::from_millis(250);
    Ok(true)
}

fn reset_runtime_after_seek(runtime: &mut DecodeRuntime) {
    runtime.demux_packet_stash = None;
    runtime.adaptive_audio_protection_until = None;
    runtime.adaptive_last_underrun_count = 0;
}

fn seek_windows_for_profile(runtime: &DecodeRuntime) -> (Duration, Duration) {
    let fps_scale = (30.0 / runtime.adaptive_profile.nominal_fps).clamp(0.75, 1.6);
    let resolution_scale = if runtime.adaptive_profile.is_high_res_video { 1.6 } else { 1.0 };
    let seek_refill_window_ms =
        ((SEEK_REFILL_WINDOW_MS_DEFAULT as f64) * resolution_scale * fps_scale).round() as u64;
    let seek_settle_window_ms =
        ((SEEK_SETTLE_WINDOW_MS_DEFAULT as f64) * resolution_scale * fps_scale).round() as u64;
    (
        Duration::from_millis(seek_refill_window_ms),
        Duration::from_millis(seek_settle_window_ms),
    )
}
