mod audio;
mod cache;
mod demux_topup;
mod video;

use super::emit_debug;
use super::DecodeRuntime;
use crate::app::media::playback::rate::{
    audio_queue_prefill_target, audio_queue_refill_floor_seconds, video_drain_batch_limit,
};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::video_pipeline::{
    drain_frames, DrainFramesContext, VideoFrameTypeMetricsRef, VideoTimestampMetricsRef,
};
use ffmpeg_next::Packet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

/// EAGAIN on video `send_packet` needs repeated drain/receive; shallow drains + low retry counts
/// falsely trip `hw_decode_fallback` (especially when audio prefill caps drains to 1 frame).
pub(super) const VIDEO_SEND_PACKET_RETRY_LIMIT: usize = 48;
pub(super) const VIDEO_SEND_PACKET_SPIN_LIMIT: usize = 4;
/// Minimum decoded frames pushed per drain pass while relieving decoder backpressure (HW stacks
/// often buffer more than one picture before accepting further packets).
pub(super) const VIDEO_DECODER_RELIEVE_MIN_FRAMES_PER_PASS: usize = 16;
pub(super) const AUDIO_SEND_PACKET_SPIN_LIMIT: usize = 2;
pub(super) const VIDEO_QUEUE_BOOST_WINDOW_MS: u64 = 320;
/// Bounded demux burst after each video packet when PCM queue is shallow (avoids long video-only stretches).
pub(super) const DEMUX_AUDIO_TOP_UP_MIN_READS: usize = 12;
pub(super) const DEMUX_AUDIO_TOP_UP_MAX_READS: usize = 96;
pub(super) const HIGH_RES_AUDIO_REFILL_FLOOR_BOOST_SECONDS: f64 = 0.05;
pub(super) const ADAPTIVE_AUDIO_PROTECTION_WINDOW_MS: u64 = 1200;
pub(super) const ADAPTIVE_AUDIO_PROTECTION_FLOOR_BOOST_SECONDS: f64 = 0.05;
pub(super) const RENDERER_QUEUE_BOOST_WINDOW_MS: u64 = 520;
pub(super) const RENDERER_QUEUE_BOOST_MIN_CAPACITY: usize = 5;
pub(super) type StopFlag = Arc<AtomicBool>;
pub(super) type TimingControlsHandle = Arc<crate::app::media::state::TimingControls>;

pub(super) fn handle_packet(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &StopFlag,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: Packet,
) -> Result<(), String> {
    track_packet_windows(runtime, &packet);
    cache::update_cache_recording(app, source, runtime, &packet)?;

    if runtime.is_video_packet(packet.stream()) {
        video::handle_video_packet(
            app,
            renderer,
            source,
            stop_flag,
            timing_controls,
            runtime,
            stream_generation,
            packet,
        )?;
        return Ok(());
    }

    audio::handle_audio_packet(
        app,
        renderer,
        stop_flag,
        timing_controls,
        runtime,
        stream_generation,
        packet,
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

pub(super) fn drain_video_frames(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &StopFlag,
    runtime: &mut DecodeRuntime,
    audio_allowed_lead_seconds: f64,
    force_decoder_relief: bool,
    stream_generation: u32,
) -> Result<(), String> {
    let high_res_video = runtime.adaptive_profile.is_high_res_video;
    let adaptive_protection_active = demux_topup::adaptive_audio_protection_active(runtime);
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
    let current_audio_queued_seconds = runtime
        .audio_pipeline
        .as_ref()
        .map(|audio| audio.output.queued_duration_seconds())
        .or(runtime.loop_state.audio_queued_seconds);
    let audio_output_paused = runtime
        .audio_pipeline
        .as_ref()
        .map(|audio| audio.output.is_paused())
        .unwrap_or(false);
    let audio_prefill_target = audio_queue_prefill_target(
        runtime.loop_state.last_applied_audio_rate,
        true,
        runtime.is_realtime_source,
        runtime.is_network_source,
    );
    let audio_refill_floor_seconds = audio_queue_refill_floor_seconds(
        runtime.loop_state.last_applied_audio_rate,
        true,
        runtime.is_realtime_source,
        runtime.is_network_source,
    )
    .unwrap_or(0.09);
    let adjusted_refill_floor_seconds = if high_res_video {
        audio_refill_floor_seconds + HIGH_RES_AUDIO_REFILL_FLOOR_BOOST_SECONDS
    } else {
        audio_refill_floor_seconds
    } + if adaptive_protection_active {
        ADAPTIVE_AUDIO_PROTECTION_FLOOR_BOOST_SECONDS
    } else {
        0.0
    };
    let audio_refill_priority_pending = matches!(
        (current_audio_queue_depth, current_audio_queued_seconds),
        (Some(depth), Some(queued_seconds))
            if depth < audio_prefill_target
                && queued_seconds + 1e-3 < adjusted_refill_floor_seconds
    );
    let max_frames_per_pass = video_drain_batch_limit(
        runtime.loop_state.last_applied_audio_rate,
        has_audio_stream,
        current_audio_queue_depth,
        runtime.is_realtime_source,
        runtime.loop_state.in_video_queue_boost(),
    )
    .map(|limit| {
        if audio_refill_priority_pending {
            if force_decoder_relief {
                limit.max(VIDEO_DECODER_RELIEVE_MIN_FRAMES_PER_PASS)
            } else {
                0
            }
        } else {
            limit
        }
    });
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
        &mut runtime.loop_state.progress_position_seconds,
        runtime.loop_state.audio_clock,
        runtime.loop_state.observed_audio_clock,
        audio_output_paused,
        runtime.loop_state.audio_queue_depth_sources,
        runtime.loop_state.audio_queued_seconds,
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

fn is_decoder_backpressure(err: &ffmpeg_next::Error) -> bool {
    matches!(
        err,
        ffmpeg_next::Error::Other { errno }
            if *errno == ffmpeg_next::util::error::EAGAIN
                || *errno == ffmpeg_next::util::error::EWOULDBLOCK
    ) || err.to_string().contains("Resource temporarily unavailable")
}

