use super::emit_debug;
use super::{drain_video_frames, refresh_tail_audio_rate, DecodeRuntime};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio_pipeline::drain_audio_frames;
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::playback::runtime::write_latest_stream_position;
use crate::app::media::state::{MediaState, TimingControls};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(crate) fn finish_decode_runtime(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    finish_cache_recording(runtime);
    flush_video_decoder(app, renderer, stop_flag, runtime, stream_generation)?;
    flush_audio_decoder(app, stop_flag, timing_controls, runtime)?;
    complete_eof_tail(
        app,
        renderer,
        stop_flag,
        timing_controls,
        runtime,
        stream_generation,
    )?;
    update_playback_progress(
        app,
        stream_generation,
        runtime.loop_state.current_position_seconds.max(0.0),
        runtime.video_ctx.duration_seconds,
        true,
    )?;
    Ok(())
}

fn finish_cache_recording(runtime: &mut DecodeRuntime) {
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        writer.finish();
    }
}

fn flush_video_decoder(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
        return Ok(());
    };
    decoder
        .send_eof()
        .map_err(|err| format!("send eof failed: {err}"))?;
    drain_video_frames(
        app,
        renderer,
        stop_flag,
        runtime,
        super::AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT,
        stream_generation,
    )
}

fn flush_audio_decoder(
    app: &AppHandle,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
) -> Result<(), String> {
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return Ok(());
    };
    audio_state
        .decoder
        .send_eof()
        .map_err(|err| format!("send audio eof failed: {err}"))?;
    drain_audio_frames(
        app,
        audio_state,
        stop_flag,
        timing_controls,
        &mut runtime.loop_state.audio_clock,
        &mut runtime.loop_state.audio_queue_depth_sources,
        &mut runtime.loop_state.active_seek_target_seconds,
    )?;
    if audio_state.stats.packets > 0 && audio_state.stats.decoded_frames == 0 {
        emit_debug(
            app,
            "audio_silent",
            format!(
                "audio packets observed ({}) but no decoded audio frames produced",
                audio_state.stats.packets
            ),
        );
    }
    Ok(())
}

fn complete_eof_tail(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    let mut tail_position_seconds = runtime.loop_state.current_position_seconds.max(0.0);
    let mut last_tail_tick = Instant::now();
    let mut last_tail_progress_emit = Instant::now() - Duration::from_millis(250);
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed during eof tail; exiting");
            return Ok(());
        }
        refresh_tail_audio_rate(runtime, timing_controls);
        let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
        let rate = timing_controls.playback_rate().max(0.25) as f64;
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(last_tail_tick);
        last_tail_tick = now;
        if duration_seconds > 0.0 {
            tail_position_seconds =
                (tail_position_seconds + elapsed.as_secs_f64() * rate).min(duration_seconds);
        }
        runtime.loop_state.current_position_seconds = tail_position_seconds;
        renderer.update_clock(tail_position_seconds, rate);
        write_latest_stream_position(&app.state::<MediaState>(), tail_position_seconds)?;
        if last_tail_progress_emit.elapsed() >= Duration::from_millis(200) {
            update_playback_progress(
                app,
                stream_generation,
                tail_position_seconds,
                duration_seconds,
                false,
            )?;
            last_tail_progress_emit = Instant::now();
        }
        let audio_done = runtime
            .audio_pipeline
            .as_ref()
            .map(|audio| audio.output.queue_depth() == 0)
            .unwrap_or(true);
        let duration_done =
            duration_seconds <= 0.0 || tail_position_seconds + 1e-3 >= duration_seconds;
        if audio_done && duration_done {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}
