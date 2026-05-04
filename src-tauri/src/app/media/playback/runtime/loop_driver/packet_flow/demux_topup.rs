use super::audio;
use super::cache;
use super::{DecodeRuntime, StopFlag, TimingControlsHandle};
use crate::app::media::playback::rate::{audio_queue_prefill_target, audio_queue_refill_floor_seconds};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::MediaState;
use ffmpeg_next::Packet;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

use super::super::pacing::should_prioritize_audio_continuity;

/// While audio output is starving, pull following mux packets before yielding to the next outer
/// iteration so audio packets are not stranded behind consecutive video GOP work.
pub(super) fn top_up_demux_audio_after_video(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &StopFlag,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    if runtime.demux_packet_stash.is_some()
        || !should_prioritize_audio_continuity(runtime)
        || runtime.audio_pipeline.is_none()
        || runtime.loop_state.pause_prefetch_mode
    {
        return Ok(());
    }
    refresh_adaptive_audio_protection(runtime);
    let media_state = app.state::<MediaState>();
    let top_up_budget = demux_audio_top_up_budget(runtime);
    for _ in 0..top_up_budget {
        if !should_prioritize_audio_continuity(runtime) {
            break;
        }
        if !media_state.runtime.stream.is_generation_current(stream_generation) {
            break;
        }
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let mut packet = Packet::empty();
        match packet.read(&mut runtime.video_ctx.input_ctx) {
            Ok(_) => {
                if runtime.is_video_packet(packet.stream()) {
                    super::track_packet_windows(runtime, &packet);
                    cache::update_cache_recording(app, source, runtime, &packet)?;
                    runtime.demux_packet_stash = Some(packet);
                    break;
                }
                super::track_packet_windows(runtime, &packet);
                cache::update_cache_recording(app, source, runtime, &packet)?;
                if runtime
                    .audio_stream_index()
                    .is_some_and(|audio_index| packet.stream() == audio_index)
                {
                    audio::handle_audio_packet(
                        app,
                        renderer,
                        stop_flag,
                        timing_controls,
                        runtime,
                        stream_generation,
                        &packet,
                    )?;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn demux_audio_top_up_budget(runtime: &DecodeRuntime) -> usize {
    let Some(audio) = runtime.audio_pipeline.as_ref() else {
        return super::DEMUX_AUDIO_TOP_UP_MIN_READS;
    };
    let has_video_stream = runtime.has_video_stream();
    let playback_rate = runtime.loop_state.last_applied_audio_rate;
    let prefill_target = audio_queue_prefill_target(
        playback_rate,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
    );
    let current_depth = audio.output.queue_depth();
    let depth_deficit = prefill_target.saturating_sub(current_depth);
    let refill_floor_seconds = audio_queue_refill_floor_seconds(
        playback_rate,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
    )
    .unwrap_or(0.09);
    let current_queued_seconds = audio.output.queued_duration_seconds();
    let seconds_deficit = (refill_floor_seconds - current_queued_seconds).max(0.0);
    let seconds_boost = (seconds_deficit / 0.03).ceil() as usize;
    let extra_audio_stream_boost = runtime
        .adaptive_profile
        .extra_audio_stream_count
        .saturating_mul(20);
    let high_res_boost = if runtime.adaptive_profile.is_high_res_video {
        16
    } else {
        0
    };
    let fps_boost = if runtime.adaptive_profile.nominal_fps >= 50.0 {
        12
    } else if runtime.adaptive_profile.nominal_fps >= 30.0 {
        4
    } else {
        0
    };
    let adaptive_protection_boost = if adaptive_audio_protection_active(runtime) {
        18
    } else {
        0
    };
    // Multi-stream muxing can interleave non-target packets; keep a moderate packet budget
    // proportional to queue deficit while capping burst cost.
    (super::DEMUX_AUDIO_TOP_UP_MIN_READS
        + depth_deficit.saturating_mul(8)
        + seconds_boost.saturating_mul(4)
        + extra_audio_stream_boost
        + high_res_boost
        + fps_boost
        + adaptive_protection_boost)
        .clamp(super::DEMUX_AUDIO_TOP_UP_MIN_READS, super::DEMUX_AUDIO_TOP_UP_MAX_READS)
}

pub(super) fn adaptive_audio_protection_active(runtime: &DecodeRuntime) -> bool {
    runtime
        .adaptive_audio_protection_until
        .map(|deadline| Instant::now() < deadline)
        .unwrap_or(false)
}

fn refresh_adaptive_audio_protection(runtime: &mut DecodeRuntime) {
    let Some(audio) = runtime.audio_pipeline.as_ref() else {
        runtime.adaptive_audio_protection_until = None;
        runtime.adaptive_last_underrun_count = 0;
        return;
    };
    let underrun_count = audio.stats.underrun_count;
    if underrun_count > runtime.adaptive_last_underrun_count {
        runtime.adaptive_last_underrun_count = underrun_count;
        let scale = if runtime.adaptive_profile.is_high_res_video {
            1.5
        } else {
            1.0
        };
        let protection_ms = ((super::ADAPTIVE_AUDIO_PROTECTION_WINDOW_MS as f64) * scale) as u64;
        runtime.adaptive_audio_protection_until =
            Some(Instant::now() + Duration::from_millis(protection_ms));
        return;
    }
    if !adaptive_audio_protection_active(runtime) {
        runtime.adaptive_audio_protection_until = None;
    }
}

