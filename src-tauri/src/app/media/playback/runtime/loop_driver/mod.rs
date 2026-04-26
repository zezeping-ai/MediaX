use super::emit::emit_debug;
use super::seek_control;
use super::video_pipeline;
use super::{
    DecodeRuntime, AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT,
};
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
    refresh_audio_rate, refresh_tail_audio_rate, should_wait_for_decode_lead,
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
        refresh_audio_rate(runtime, timing_controls);
        if should_wait_for_decode_lead(runtime) {
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
            Err(FfmpegError::Eof) => {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                if runtime.should_tail_eof {
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
                break;
            }
            Err(_) => {
                if runtime.should_tail_eof {
                    std::thread::sleep(Duration::from_millis(50));
                }
                continue;
            }
        }
    }
    Ok(())
}

fn should_exit_loop(
    app: &AppHandle,
    stop_flag: &Arc<AtomicBool>,
    stream_generation: u32,
) -> bool {
    if !app
        .state::<MediaState>()
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
        &mut runtime.video_ctx.decoder,
        target_seconds,
        &mut runtime.loop_state.playback_clock,
        &mut runtime.loop_state.current_position_seconds,
        runtime.audio_pipeline.as_mut(),
    )?;
    renderer.reset_timeline(
        target_seconds.max(0.0),
        timing_controls.playback_rate() as f64,
    );
    runtime.loop_state.audio_clock = None;
    runtime.loop_state.audio_queue_depth_sources = None;
    runtime.loop_state.active_seek_target_seconds = Some(target_seconds.max(0.0));
    runtime.loop_state.last_video_pts_seconds = None;
    runtime.loop_state.last_progress_emit = Instant::now() - Duration::from_millis(250);
    Ok(true)
}
