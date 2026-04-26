use super::emit_debug;
use super::pacing::current_audio_allowed_lead_seconds;
use super::DecodeRuntime;
use crate::app::media::playback::runtime::audio_pipeline::drain_audio_frames;
use crate::app::media::playback::runtime::session::{
    current_recording_target, update_cache_session_error, CacheRemuxWriter,
};
use crate::app::media::playback::runtime::video_pipeline::{drain_frames, DrainFramesContext};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::TimingControls;
use ffmpeg_next::Packet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

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
    if packet.stream() == runtime.video_ctx.video_stream_index
        || audio_stream_index.is_some_and(|index| packet.stream() == index)
    {
        runtime
            .loop_state
            .update_media_required_window(packet.size());
    }
    update_cache_recording(app, source, runtime, &packet)?;
    if packet.stream() == runtime.video_ctx.video_stream_index {
        runtime
            .loop_state
            .record_video_packet(app, &packet, runtime.video_ctx.video_time_base);
        runtime.video_ctx.decoder.send_packet(&packet).map_err(|err| {
            emit_debug(
                app,
                "decode_error_detail",
                format!("video_send_packet_failed err={err}"),
            );
            format!("send packet failed: {err}")
        })?;
        drain_video_frames(
            app,
            renderer,
            stop_flag,
            runtime,
            current_audio_allowed_lead_seconds(runtime),
            stream_generation,
        )?;
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
    let mut drain_ctx = DrainFramesContext {
        app,
        renderer,
        decoder: &mut runtime.video_ctx.decoder,
        video_time_base: runtime.video_ctx.video_time_base,
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
        stream_generation,
    };
    drain_frames(&mut drain_ctx)
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
