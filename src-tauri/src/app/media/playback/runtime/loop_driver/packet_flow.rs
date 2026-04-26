use super::emit_debug;
use super::pacing::current_audio_allowed_lead_seconds;
use super::DecodeRuntime;
use crate::app::media::playback::runtime::audio_pipeline::drain_audio_frames;
use crate::app::media::playback::runtime::session::{
    current_recording_target, update_cache_session_error, CacheRemuxWriter,
};
use crate::app::media::playback::runtime::video_pipeline::{drain_frames, DrainFramesContext};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::{progress::update_playback_progress, write_latest_stream_position};
use crate::app::media::state::TimingControls;
use ffmpeg_next::Packet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

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
    runtime.loop_state.update_network_window(packet.size());
    let audio_stream_index = runtime.audio_pipeline.as_ref().map(|audio| audio.stream_index);
    if runtime
        .video_ctx
        .video_stream_index
        .is_some_and(|index| packet.stream() == index)
        || audio_stream_index.is_some_and(|index| packet.stream() == index)
    {
        runtime
            .loop_state
            .update_media_required_window(packet.size());
    }
    update_cache_recording(app, source, runtime, &packet)?;
    if runtime
        .video_ctx
        .video_stream_index
        .is_some_and(|index| packet.stream() == index)
    {
        let Some(video_time_base) = runtime.video_ctx.video_time_base else {
            return Ok(());
        };
        let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
            return Ok(());
        };
        runtime
            .loop_state
            .record_video_packet(app, &packet, video_time_base);
        if let Err(err) = decoder.send_packet(&packet) {
            runtime.loop_state.video_packet_soft_error_count =
                runtime.loop_state.video_packet_soft_error_count.saturating_add(1);
            emit_debug(
                app,
                "decode_error_detail",
                format!("video_send_packet_failed err={err}"),
            );
            emit_debug(
                app,
                "decode_recovery",
                "video send_packet soft error ignored; preserving decoder state for later frames",
            );
            return Ok(());
        }
        if let Err(err) = drain_video_frames(
            app,
            renderer,
            stop_flag,
            runtime,
            current_audio_allowed_lead_seconds(runtime),
            stream_generation,
        ) {
            runtime.loop_state.video_packet_soft_error_count =
                runtime.loop_state.video_packet_soft_error_count.saturating_add(1);
            emit_debug(
                app,
                "decode_error_detail",
                format!("video_drain_failed err={err}"),
            );
            emit_debug(
                app,
                "decode_recovery",
                "video drain soft error ignored; preserving decoder state for later frames",
            );
            runtime.loop_state.last_video_pts_seconds = None;
        }
        return Ok(());
    }
    if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
        if packet.stream() == audio_state.stream_index {
            audio_state.decoder.send_packet(&packet).map_err(|err| {
                emit_debug(
                    app,
                    "decode_error_detail",
                    format!("audio_send_packet_failed err={err}"),
                );
                format!("send audio packet failed: {err}")
            })?;
            drain_audio_frames(
                app,
                audio_state,
                stop_flag,
                timing_controls,
                &mut runtime.loop_state.audio_clock,
                &mut runtime.loop_state.audio_queue_depth_sources,
                &mut runtime.loop_state.active_seek_target_seconds,
            )?;
            if runtime.video_ctx.video_stream_index.is_none() {
                update_audio_only_progress(app, renderer, timing_controls, runtime, stream_generation)?;
            }
        }
    }
    Ok(())
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
    let mut drain_ctx = DrainFramesContext {
        app,
        renderer,
        decoder,
        video_time_base,
        scaler: &mut runtime.scaler,
        duration_seconds: runtime.video_ctx.duration_seconds,
        output_width: runtime.video_ctx.output_width,
        output_height: runtime.video_ctx.output_height,
        stop_flag,
        playback_clock: &mut runtime.loop_state.playback_clock,
        last_progress_emit: &mut runtime.loop_state.last_progress_emit,
        current_position_seconds: &mut runtime.loop_state.current_position_seconds,
        audio_clock: runtime.loop_state.audio_clock,
        audio_queue_depth_sources: runtime.loop_state.audio_queue_depth_sources,
        active_seek_target_seconds: &mut runtime.loop_state.active_seek_target_seconds,
        last_video_pts_seconds: &mut runtime.loop_state.last_video_pts_seconds,
        fps_window: &mut runtime.loop_state.fps_window,
        frame_pipeline: &mut runtime.loop_state.frame_pipeline,
        process_metrics: &mut runtime.loop_state.process_metrics,
        audio_allowed_lead_seconds,
        network_read_bps: runtime.loop_state.net_read_bps,
        media_required_bps: runtime.loop_state.media_required_bps,
        video_ts_window_start: &mut runtime.loop_state.video_ts_window_start,
        video_ts_samples: &mut runtime.loop_state.video_ts_samples,
        video_pts_missing: &mut runtime.loop_state.video_pts_missing,
        video_pts_backtrack: &mut runtime.loop_state.video_pts_backtrack,
        video_pts_jitter_abs_sum_ms: &mut runtime.loop_state.video_pts_jitter_abs_sum_ms,
        video_pts_jitter_max_ms: &mut runtime.loop_state.video_pts_jitter_max_ms,
        video_frame_type_window_start: &mut runtime.loop_state.video_frame_type_window_start,
        video_frame_type_i: &mut runtime.loop_state.video_frame_type_i,
        video_frame_type_p: &mut runtime.loop_state.video_frame_type_p,
        video_frame_type_b: &mut runtime.loop_state.video_frame_type_b,
        video_frame_type_other: &mut runtime.loop_state.video_frame_type_other,
        video_packet_soft_error_count: &mut runtime.loop_state.video_packet_soft_error_count,
        stream_generation,
    };
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
    write_latest_stream_position(&app.state::<crate::app::media::state::MediaState>(), runtime.loop_state.current_position_seconds)?;
    if runtime.loop_state.last_progress_emit.elapsed() >= Duration::from_millis(200) {
        update_playback_progress(
            app,
            stream_generation,
            runtime.loop_state.current_position_seconds,
            duration_seconds,
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
    match (
        runtime.loop_state.cache_writer.as_ref(),
        recording_target.as_ref(),
    ) {
        (None, Some(target)) => match CacheRemuxWriter::new(&runtime.video_ctx.input_ctx, target) {
            Ok(writer) => {
                emit_debug(app, "cache_recording", format!("start remux recording: {target}"));
                runtime.loop_state.cache_writer = Some(writer);
            }
            Err(err) => {
                update_cache_session_error(app, source, err.clone());
                emit_debug(app, "cache_recording_error", err);
            }
        },
        (Some(writer), Some(target)) if writer.output_path != *target => {
            if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
                writer.finish();
            }
            runtime.loop_state.cache_writer = None;
        }
        (Some(_), None) => {
            if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
                writer.finish();
            }
            runtime.loop_state.cache_writer = None;
        }
        _ => {}
    }
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
