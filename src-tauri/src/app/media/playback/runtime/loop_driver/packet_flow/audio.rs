use super::{is_decoder_backpressure, DecodeRuntime, StopFlag, TimingControlsHandle};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio_pipeline::{
    drain_audio_frames, AudioDrainParams, AudioDrainStateRefs,
};
use ffmpeg_next::Packet;
use std::time::Duration;
use tauri::AppHandle;

use super::super::emit_debug;
use super::super::progress_emit::write_position_and_maybe_emit_progress;

pub(super) fn handle_audio_packet(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &StopFlag,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
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
    let base_params = AudioDrainParams {
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
    if packet.stream() != audio_state.stream_index {
        return Ok(());
    }

    send_audio_packet_with_backpressure_recovery(
        app,
        audio_state,
        packet,
        AudioDrainStateRefs {
            stop_flag,
            audio_clock: &mut runtime.loop_state.audio_clock,
            observed_audio_clock: &mut runtime.loop_state.observed_audio_clock,
            audio_queue_depth_sources: &mut runtime.loop_state.audio_queue_depth_sources,
            audio_queued_seconds: &mut runtime.loop_state.audio_queued_seconds,
            active_seek_target_seconds: &mut runtime.loop_state.active_seek_target_seconds,
        },
        base_params,
    )?;

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
        base_params,
    )?;

    if seeking_low_latency_refill && audio_state.stats.seek_refill_logged {
        runtime.loop_state.clear_seek_refill();
    }
    if !has_video_stream {
        update_audio_only_progress(app, renderer, timing_controls, runtime, stream_generation)?;
    }
    Ok(())
}

fn send_audio_packet_with_backpressure_recovery(
    app: &AppHandle,
    audio_state: &mut crate::app::media::playback::runtime::audio_pipeline::AudioPipeline,
    packet: &Packet,
    state_refs: AudioDrainStateRefs<'_>,
    params: AudioDrainParams,
) -> Result<(), String> {
    let AudioDrainStateRefs {
        stop_flag,
        audio_clock,
        observed_audio_clock,
        audio_queue_depth_sources,
        audio_queued_seconds,
        active_seek_target_seconds,
    } = state_refs;
    let mut attempts = 0usize;
    loop {
        match audio_state.decoder.send_packet(packet) {
            Ok(()) => return Ok(()),
            Err(err)
                if is_decoder_backpressure(&err)
                    && attempts < super::AUDIO_SEND_PACKET_RETRY_LIMIT =>
            {
                attempts = attempts.saturating_add(1);
                if attempts == 1 {
                    emit_debug(
                        app,
                        "audio_decoder_backpressure",
                        format!(
                            "audio send_packet would block; draining decoded audio before retry #{attempts}"
                        ),
                    );
                }
                drain_audio_frames(
                    app,
                    audio_state,
                    AudioDrainStateRefs {
                        stop_flag,
                        audio_clock: &mut *audio_clock,
                        observed_audio_clock: &mut *observed_audio_clock,
                        audio_queue_depth_sources: &mut *audio_queue_depth_sources,
                        audio_queued_seconds: &mut *audio_queued_seconds,
                        active_seek_target_seconds: &mut *active_seek_target_seconds,
                    },
                    AudioDrainParams {
                        decoder_relief_mode: true,
                        ..params
                    },
                )?;
            }
            Err(err) => {
                emit_debug(
                    app,
                    "decode_error_detail",
                    format!("audio_send_packet_failed err={err}"),
                );
                return Err(format!("send audio packet failed: {err}"));
            }
        }
    }
}

fn update_audio_only_progress(
    app: &AppHandle,
    renderer: &RendererState,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
    let mut position_seconds = runtime
        .loop_state
        .audio_clock
        .map(|clock| clock.now_seconds())
        .unwrap_or(runtime.loop_state.progress_position_seconds);
    if duration_seconds > 0.0 {
        position_seconds = position_seconds.min(duration_seconds);
    }
    let normalized = position_seconds.max(0.0);
    renderer.update_clock(normalized, timing_controls.playback_rate() as f64);
    let _ = write_position_and_maybe_emit_progress(
        app,
        runtime,
        stream_generation,
        normalized,
        false,
        Duration::from_millis(200),
        true,
    )?;
    Ok(())
}

