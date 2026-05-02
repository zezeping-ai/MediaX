use super::emit_debug;
use super::pacing::current_audio_allowed_lead_seconds;
use super::DecodeRuntime;
use crate::app::media::playback::rate::video_drain_batch_limit;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio_pipeline::drain_audio_frames;
use crate::app::media::playback::runtime::session::{
    current_recording_target, update_cache_session_error, CacheRemuxWriter,
};
use crate::app::media::playback::runtime::video_pipeline::{
    drain_frames, DrainFramesContext, VideoFrameTypeMetricsRef, VideoTimestampMetricsRef,
};
use crate::app::media::playback::runtime::{
    progress::{resolve_buffered_position_seconds, update_playback_progress},
    write_latest_stream_position,
};
use crate::app::media::state::TimingControls;
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const VIDEO_SEND_PACKET_RETRY_LIMIT: usize = 3;
const AUDIO_SEND_PACKET_RETRY_LIMIT: usize = 3;

pub(super) fn handle_packet(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: Packet,
) -> Result<(), String> {
    track_packet_windows(runtime, &packet);
    update_cache_recording(app, source, runtime, &packet)?;

    if is_video_packet(runtime, &packet) {
        handle_video_packet(
            app,
            renderer,
            stop_flag,
            runtime,
            stream_generation,
            &packet,
        )?;
        return Ok(());
    }

    handle_audio_packet(
        app,
        renderer,
        stop_flag,
        timing_controls,
        runtime,
        stream_generation,
        &packet,
    )
}

pub(super) fn track_packet_windows(runtime: &mut DecodeRuntime, packet: &Packet) {
    runtime.loop_state.update_network_window(packet.size());
    if runtime.is_video_packet(packet.stream())
        || runtime
            .audio_stream_index()
            .is_some_and(|index| packet.stream() == index)
    {
        runtime
            .loop_state
            .update_media_required_window(packet.size());
    }
}

fn is_video_packet(runtime: &DecodeRuntime, packet: &Packet) -> bool {
    runtime.is_video_packet(packet.stream())
}

fn handle_video_packet(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    let Some(video_time_base) = runtime.video_ctx.video_time_base else {
        return Ok(());
    };
    runtime
        .loop_state
        .record_video_packet(app, packet, video_time_base);
    send_video_packet_with_backpressure_recovery(
        app,
        renderer,
        stop_flag,
        runtime,
        stream_generation,
        packet,
    )?;
    drain_video_frames(
        app,
        renderer,
        stop_flag,
        runtime,
        current_audio_allowed_lead_seconds(runtime),
        stream_generation,
    )
    .map_err(|err| handle_video_soft_error(app, runtime, "video_drain_failed", err))?;
    Ok(())
}

fn send_video_packet_with_backpressure_recovery(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    let mut attempts = 0usize;
    loop {
        match send_video_packet_once(runtime, packet) {
            Ok(()) => return Ok(()),
            Err(err) if is_decoder_backpressure(&err) && attempts < VIDEO_SEND_PACKET_RETRY_LIMIT => {
                attempts = attempts.saturating_add(1);
                emit_debug(
                    app,
                    "video_decoder_backpressure",
                    format!("send_packet would block; draining decoded frames before retry #{attempts}"),
                );
                drain_video_frames(
                    app,
                    renderer,
                    stop_flag,
                    runtime,
                    current_audio_allowed_lead_seconds(runtime),
                    stream_generation,
                )
                .map_err(|drain_err| {
                    handle_video_soft_error(app, runtime, "video_drain_failed", drain_err)
                })?;
            }
            Err(err) => {
                return Err(handle_video_soft_error(
                    app,
                    runtime,
                    "video_send_packet_failed",
                    err.to_string(),
                ));
            }
        }
    }
}

fn send_video_packet_once(runtime: &mut DecodeRuntime, packet: &Packet) -> Result<(), FfmpegError> {
    let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
        return Ok(());
    };
    decoder.send_packet(packet)
}

fn is_decoder_backpressure(err: &FfmpegError) -> bool {
    matches!(
        err,
        FfmpegError::Other { errno }
            if *errno == ffmpeg_next::util::error::EAGAIN
                || *errno == ffmpeg_next::util::error::EWOULDBLOCK
    ) || err.to_string().contains("Resource temporarily unavailable")
}

fn handle_video_soft_error(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    stage: &str,
    err: impl Into<String>,
) -> String {
    let message = err.into();
    runtime.loop_state.increment_video_soft_error_count();
    emit_debug(app, "decode_error_detail", format!("{stage} err={message}"));
    emit_debug(
        app,
        "decode_recovery",
        "video soft error ignored; preserving decoder state for later frames",
    );
    if stage == "video_drain_failed" {
        runtime.loop_state.last_video_pts_seconds = None;
    }
    message
}

fn handle_audio_packet(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    let has_video_stream = runtime.has_video_stream();
    let force_low_latency_output = runtime.loop_state.in_rate_switch_settle();
    let building_rate_switch_cover =
        runtime.loop_state.pending_audio_rate.is_some() && !force_low_latency_output;
    let seeking_low_latency_refill = runtime.loop_state.in_seek_refill();
    let in_seek_settle = runtime.loop_state.in_seek_settle();
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return Ok(());
    };
    if packet.stream() != audio_state.stream_index {
        return Ok(());
    }
    send_audio_packet_with_backpressure_recovery(
        app,
        stop_flag,
        audio_state,
        packet,
        runtime.loop_state.last_applied_audio_rate,
        &mut runtime.loop_state.audio_clock,
        &mut runtime.loop_state.audio_queue_depth_sources,
        &mut runtime.loop_state.audio_queued_seconds,
        &mut runtime.loop_state.active_seek_target_seconds,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
        building_rate_switch_cover,
        seeking_low_latency_refill,
        in_seek_settle,
        force_low_latency_output,
    )?;
    drain_audio_frames(
        app,
        audio_state,
        stop_flag,
        runtime.loop_state.last_applied_audio_rate,
        &mut runtime.loop_state.audio_clock,
        &mut runtime.loop_state.audio_queue_depth_sources,
        &mut runtime.loop_state.audio_queued_seconds,
        &mut runtime.loop_state.active_seek_target_seconds,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
        building_rate_switch_cover,
        seeking_low_latency_refill,
        in_seek_settle,
        false,
        force_low_latency_output,
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
    stop_flag: &Arc<AtomicBool>,
    audio_state: &mut crate::app::media::playback::runtime::audio_pipeline::AudioPipeline,
    packet: &Packet,
    applied_playback_rate: crate::app::media::playback::rate::PlaybackRate,
    audio_clock: &mut Option<crate::app::media::playback::runtime::clock::AudioClock>,
    audio_queue_depth_sources: &mut Option<usize>,
    audio_queued_seconds: &mut Option<f64>,
    active_seek_target_seconds: &mut Option<f64>,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    building_rate_switch_cover: bool,
    seeking_low_latency_refill: bool,
    in_seek_settle: bool,
    force_low_latency_output: bool,
) -> Result<(), String> {
    let mut attempts = 0usize;
    loop {
        match audio_state.decoder.send_packet(packet) {
            Ok(()) => return Ok(()),
            Err(err) if is_decoder_backpressure(&err) && attempts < AUDIO_SEND_PACKET_RETRY_LIMIT => {
                attempts = attempts.saturating_add(1);
                emit_debug(
                    app,
                    "audio_decoder_backpressure",
                    format!("audio send_packet would block; draining decoded audio before retry #{attempts}"),
                );
                drain_audio_frames(
                    app,
                    audio_state,
                    stop_flag,
                    applied_playback_rate,
                    audio_clock,
                    audio_queue_depth_sources,
                    audio_queued_seconds,
                    active_seek_target_seconds,
                    has_video_stream,
                    is_realtime_source,
                    is_network_source,
                    building_rate_switch_cover,
                    seeking_low_latency_refill,
                    in_seek_settle,
                    true,
                    force_low_latency_output,
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

pub(super) fn drain_video_frames(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    runtime: &mut DecodeRuntime,
    audio_allowed_lead_seconds: f64,
    stream_generation: u32,
) -> Result<(), String> {
    let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
        return Ok(());
    };
    let Some(video_time_base) = runtime.video_ctx.video_time_base else {
        return Ok(());
    };
    let network_read_bps = runtime.loop_state.network_read_bps();
    let media_required_bps = runtime.loop_state.media_required_bps();
    let has_audio_stream = runtime.audio_pipeline.is_some();
    let current_audio_queue_depth = runtime
        .audio_pipeline
        .as_ref()
        .map(|audio| audio.output.queue_depth())
        .or(runtime.loop_state.audio_queue_depth_sources);
    let max_frames_per_pass = video_drain_batch_limit(
        runtime.loop_state.last_applied_audio_rate,
        has_audio_stream,
        current_audio_queue_depth,
        runtime.is_realtime_source,
    );
    let mut drain_ctx = DrainFramesContext::new(
        app,
        renderer,
        &runtime.video_ctx.input_ctx,
        decoder,
        video_time_base,
        &mut runtime.scaler,
        runtime.video_ctx.duration_seconds,
        runtime.video_ctx.output_width,
        runtime.video_ctx.output_height,
        stop_flag,
        &mut runtime.loop_state.playback_clock,
        &mut runtime.loop_state.last_progress_emit,
        &mut runtime.loop_state.current_position_seconds,
        runtime.loop_state.audio_clock,
        runtime.loop_state.audio_queue_depth_sources,
        &mut runtime.loop_state.active_seek_target_seconds,
        &mut runtime.loop_state.last_video_pts_seconds,
        &mut runtime.loop_state.fps_window,
        &mut runtime.loop_state.frame_pipeline,
        &mut runtime.loop_state.process_metrics,
        audio_allowed_lead_seconds,
        network_read_bps,
        media_required_bps,
        runtime.is_network_source,
        runtime.is_realtime_source,
        VideoTimestampMetricsRef {
            window_start: &mut runtime.loop_state.video_timestamp_metrics.window_start,
            samples: &mut runtime.loop_state.video_timestamp_metrics.samples,
            pts_missing: &mut runtime.loop_state.video_timestamp_metrics.pts_missing,
            pts_backtrack: &mut runtime.loop_state.video_timestamp_metrics.pts_backtrack,
            pts_jitter_abs_sum_ms: &mut runtime
                .loop_state
                .video_timestamp_metrics
                .pts_jitter_abs_sum_ms,
            pts_jitter_max_ms: &mut runtime.loop_state.video_timestamp_metrics.pts_jitter_max_ms,
            last_gap_seconds: &mut runtime.loop_state.video_timestamp_metrics.last_gap_seconds,
        },
        VideoFrameTypeMetricsRef {
            window_start: &mut runtime.loop_state.video_frame_type_metrics.window_start,
            i_count: &mut runtime.loop_state.video_frame_type_metrics.i_count,
            p_count: &mut runtime.loop_state.video_frame_type_metrics.p_count,
            b_count: &mut runtime.loop_state.video_frame_type_metrics.b_count,
            other_count: &mut runtime.loop_state.video_frame_type_metrics.other_count,
        },
        &mut runtime.loop_state.video_packet_metrics.soft_error_count,
        stream_generation,
        max_frames_per_pass,
    );
    drain_frames(&mut drain_ctx)
}

fn update_audio_only_progress(
    app: &AppHandle,
    renderer: &RendererState,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
    let mut position_seconds = runtime
        .loop_state
        .audio_clock
        .map(|clock| clock.now_seconds())
        .unwrap_or(runtime.loop_state.current_position_seconds);
    if duration_seconds > 0.0 {
        position_seconds = position_seconds.min(duration_seconds);
    }
    runtime.loop_state.current_position_seconds = position_seconds.max(0.0);
    renderer.update_clock(
        runtime.loop_state.current_position_seconds,
        timing_controls.playback_rate() as f64,
    );
    write_latest_stream_position(
        &app.state::<crate::app::media::state::MediaState>(),
        runtime.loop_state.current_position_seconds,
    )?;
    if runtime.loop_state.last_progress_emit.elapsed() >= Duration::from_millis(200) {
        let buffered_position_seconds = resolve_buffered_position_seconds(
            &runtime.video_ctx.input_ctx,
            duration_seconds,
            runtime.loop_state.current_position_seconds,
            runtime.is_network_source,
            runtime.is_realtime_source,
        );
        update_playback_progress(
            app,
            stream_generation,
            runtime.loop_state.current_position_seconds,
            duration_seconds,
            buffered_position_seconds,
            false,
        )?;
        runtime.loop_state.last_progress_emit = std::time::Instant::now();
    }
    Ok(())
}

fn update_cache_recording(
    app: &AppHandle,
    source: &str,
    runtime: &mut DecodeRuntime,
    packet: &Packet,
) -> Result<(), String> {
    let recording_target = current_recording_target(app, source)?;
    sync_cache_writer_target(app, source, runtime, recording_target.as_deref());
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        if let Err(err) = writer.write_packet(&runtime.video_ctx.input_ctx, packet) {
            writer.finish();
            runtime.loop_state.cache_writer = None;
            update_cache_session_error(app, source, err.to_string());
            emit_debug(app, "cache_recording_error", err);
        }
    }
    Ok(())
}

fn sync_cache_writer_target(
    app: &AppHandle,
    source: &str,
    runtime: &mut DecodeRuntime,
    recording_target: Option<&str>,
) {
    match (runtime.loop_state.cache_writer.as_ref(), recording_target) {
        (None, Some(target)) => start_cache_writer(app, source, runtime, target),
        (Some(writer), Some(target)) if writer.output_path != target => {
            finish_cache_writer(runtime);
        }
        (Some(_), None) => {
            finish_cache_writer(runtime);
        }
        _ => {}
    }
}

fn start_cache_writer(app: &AppHandle, source: &str, runtime: &mut DecodeRuntime, target: &str) {
    match CacheRemuxWriter::new(&runtime.video_ctx.input_ctx, target) {
        Ok(writer) => {
            emit_debug(
                app,
                "cache_recording",
                format!("start remux recording: {target}"),
            );
            runtime.loop_state.cache_writer = Some(writer);
        }
        Err(err) => {
            update_cache_session_error(app, source, err.clone());
            emit_debug(app, "cache_recording_error", err);
        }
    }
}

fn finish_cache_writer(runtime: &mut DecodeRuntime) {
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        writer.finish();
    }
    runtime.loop_state.cache_writer = None;
}
