use super::emit_debug;
use super::progress_emit::write_position_and_maybe_emit_progress;
use super::sleep_with_stop_flag;
use super::{drain_video_frames, refresh_tail_audio_rate, DecodeRuntime};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio::effective_playback_rate;
use crate::app::media::playback::runtime::audio_pipeline::{
    drain_audio_frames, AudioDrainParams, AudioDrainStateRefs,
};
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::state::TimingControls;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

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
        runtime.loop_state.progress_position_seconds.max(0.0),
        runtime.video_ctx.duration_seconds,
        runtime.video_ctx.duration_seconds.max(0.0),
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
        true,
        stream_generation,
    )
}

fn flush_audio_decoder(
    app: &AppHandle,
    stop_flag: &Arc<AtomicBool>,
    _timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
) -> Result<(), String> {
    let has_video_stream = runtime.has_video_stream();
    let video_frame_duration_seconds =
        has_video_stream.then(|| runtime.loop_state.playback_clock.frame_duration_seconds());
    let force_low_latency_output = runtime.loop_state.in_rate_switch_settle();
    let building_rate_switch_cover =
        runtime.loop_state.pending_audio_rate.is_some() && !force_low_latency_output;
    let seeking_low_latency_refill = runtime.loop_state.in_seek_refill();
    let in_seek_settle = runtime.loop_state.in_seek_settle();
    let audio_sync_warmup_factor = runtime.loop_state.audio_sync_warmup_factor();
    let params = AudioDrainParams {
        applied_playback_rate: runtime.loop_state.last_applied_audio_rate,
        has_video_stream,
        is_realtime_source: runtime.is_realtime_source,
        is_network_source: runtime.is_network_source,
        building_rate_switch_cover,
        seeking_low_latency_refill,
        in_seek_settle,
        audio_sync_warmup_factor,
        decoder_relief_mode: false,
        force_low_latency_output,
        video_frame_duration_seconds,
    };
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
        AudioDrainStateRefs {
            stop_flag,
            audio_clock: &mut runtime.loop_state.audio_clock,
            observed_audio_clock: &mut runtime.loop_state.observed_audio_clock,
            audio_queue_depth_sources: &mut runtime.loop_state.audio_queue_depth_sources,
            audio_queued_seconds: &mut runtime.loop_state.audio_queued_seconds,
            active_seek_target_seconds: &mut runtime.loop_state.active_seek_target_seconds,
        },
        params,
    )?;
    if seeking_low_latency_refill && audio_state.stats.seek_refill_logged {
        runtime.loop_state.clear_seek_refill();
    }
    if let Some((channels, block)) = audio_state.flush_staged_output_pcm() {
        audio_state.stats.queued_samples = audio_state
            .stats
            .queued_samples
            .saturating_add(block.len() as u64);
        audio_state.output.append_pcm_f32_owned(
            audio_state.decoder.rate(),
            channels,
            block,
            None,
            0.0,
        );
        runtime.loop_state.audio_queue_depth_sources = Some(audio_state.output.queue_depth());
        runtime.loop_state.audio_queued_seconds =
            Some(audio_state.output.queued_duration_seconds());
    }
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
    let mut tail_position_seconds = runtime.loop_state.progress_position_seconds.max(0.0);
    let mut last_tail_tick = Instant::now();
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed during eof tail; exiting");
            return Ok(());
        }
        refresh_tail_audio_rate(runtime, timing_controls);
        let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
        let rate = effective_playback_rate(
            timing_controls.playback_rate_value(),
            runtime.is_realtime_source,
        )
        .as_f64();
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(last_tail_tick);
        last_tail_tick = now;
        if duration_seconds > 0.0 {
            tail_position_seconds =
                (tail_position_seconds + elapsed.as_secs_f64() * rate).min(duration_seconds);
        }
        renderer.update_clock(tail_position_seconds, rate);
        write_position_and_maybe_emit_progress(
            app,
            runtime,
            stream_generation,
            tail_position_seconds,
            false,
            Duration::from_millis(200),
            true,
        )?;
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
        sleep_with_stop_flag(stop_flag, Duration::from_millis(20));
    }
    Ok(())
}
