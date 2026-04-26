mod frame_pipeline;
mod process_metrics;

use super::{
    emit_debug, emit_telemetry_payloads, write_latest_stream_position, METRICS_EMIT_INTERVAL_MS,
};
use crate::app::media::playback::events::{
    MediaDecodeQuantileStats, MediaFrameTypeStats, MediaTelemetryPayload,
    MediaVideoTimestampStats,
};
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::render::video_frame::{
    ensure_scaler, transfer_hw_frame_if_needed, ScalerSpec,
};
use crate::app::media::playback::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::state::MediaState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(super) use frame_pipeline::VideoFramePipeline;
pub(super) use process_metrics::ProcessMetricsSampler;

pub(super) struct DrainFramesContext<'a> {
    pub app: &'a AppHandle,
    pub renderer: &'a RendererState,
    pub decoder: &'a mut ffmpeg::decoder::Video,
    pub video_time_base: ffmpeg::Rational,
    pub scaler: &'a mut Option<ScalingContext>,
    pub duration_seconds: f64,
    pub output_width: u32,
    pub output_height: u32,
    pub stop_flag: &'a std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub playback_clock: &'a mut PlaybackClock,
    pub last_progress_emit: &'a mut Instant,
    pub current_position_seconds: &'a mut f64,
    pub audio_clock: Option<AudioClock>,
    pub audio_queue_depth_sources: Option<usize>,
    pub active_seek_target_seconds: &'a mut Option<f64>,
    pub last_video_pts_seconds: &'a mut Option<f64>,
    pub fps_window: &'a mut FpsWindow,
    pub frame_pipeline: &'a mut VideoFramePipeline,
    pub process_metrics: &'a mut ProcessMetricsSampler,
    pub audio_allowed_lead_seconds: f64,
    pub network_read_bps: Option<f64>,
    pub media_required_bps: Option<f64>,
    pub video_ts_window_start: &'a mut Instant,
    pub video_ts_samples: &'a mut u64,
    pub video_pts_missing: &'a mut u64,
    pub video_pts_backtrack: &'a mut u64,
    pub video_pts_jitter_abs_sum_ms: &'a mut f64,
    pub video_pts_jitter_max_ms: &'a mut f64,
    pub video_frame_type_window_start: &'a mut Instant,
    pub video_frame_type_i: &'a mut u64,
    pub video_frame_type_p: &'a mut u64,
    pub video_frame_type_b: &'a mut u64,
    pub video_frame_type_other: &'a mut u64,
    pub video_packet_soft_error_count: &'a mut u64,
    pub stream_generation: u32,
}

pub(super) fn drain_frames(ctx: &mut DrainFramesContext<'_>) -> Result<(), String> {
    let mut decoded = frame::Video::empty();
    while let Ok(()) = ctx.decoder.receive_frame(&mut decoded) {
        let frame_cost_start = Instant::now();
        if !ctx
            .app
            .state::<MediaState>()
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let pict_type = unsafe { (*decoded.as_ptr()).pict_type };
        if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_I {
            *ctx.video_frame_type_i = ctx.video_frame_type_i.saturating_add(1);
        } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_P {
            *ctx.video_frame_type_p = ctx.video_frame_type_p.saturating_add(1);
        } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_B {
            *ctx.video_frame_type_b = ctx.video_frame_type_b.saturating_add(1);
        } else {
            *ctx.video_frame_type_other = ctx.video_frame_type_other.saturating_add(1);
        }
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        *ctx.video_ts_samples = ctx.video_ts_samples.saturating_add(1);
        if decoded.pts().is_none() {
            *ctx.video_pts_missing = ctx.video_pts_missing.saturating_add(1);
        }
        let hinted_valid = hinted_seconds.filter(|value| value.is_finite() && *value >= 0.0);
        if let (Some(target), Some(hint)) = (*ctx.active_seek_target_seconds, hinted_valid) {
            if hint + 0.03 < target {
                continue;
            }
            *ctx.active_seek_target_seconds = None;
        }

        while !ctx.renderer.can_accept_frame() {
            if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(RENDER_BACKPRESSURE_SLEEP_MS));
        }

        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(err) => {
                ctx.frame_pipeline.on_hw_transfer_failed(ctx.app, &err);
                continue;
            }
        };
        if let Err(err) = ensure_scaler(
            ctx.scaler,
            ScalerSpec {
                src_format: frame_for_scale.format(),
                src_width: frame_for_scale.width(),
                src_height: frame_for_scale.height(),
                dst_format: format::pixel::Pixel::YUV420P,
                dst_width: ctx.output_width,
                dst_height: ctx.output_height,
                flags: Flags::BILINEAR,
            },
        ) {
            ctx.frame_pipeline.on_scale_failed(ctx.app, &err);
            continue;
        }
        let mut nv12_frame = frame::Video::empty();
        if let Some(scaler) = ctx.scaler.as_mut() {
            if let Err(err) = scaler.run(&frame_for_scale, &mut nv12_frame) {
                ctx.frame_pipeline
                    .on_scale_failed(ctx.app, &format!("scale frame failed: {err}"));
                continue;
            }
        }
        let _ = ctx.frame_pipeline.resolve_color_profile(ctx.app, &nv12_frame);
        let audio_now_seconds = ctx.audio_clock.map(|clock| clock.now_seconds());
        let position_seconds = ctx.playback_clock.tick(
            hinted_seconds,
            audio_now_seconds,
            ctx.audio_queue_depth_sources,
            ctx.audio_allowed_lead_seconds,
        );
        let estimated_pts = hinted_valid.unwrap_or_else(|| {
            if let Some(previous_pts) = *ctx.last_video_pts_seconds {
                previous_pts + ctx.playback_clock.frame_duration.as_secs_f64()
            } else {
                position_seconds.max(0.0)
            }
        });
        if let Some(previous_pts) = *ctx.last_video_pts_seconds {
            let gap = estimated_pts - previous_pts;
            let expected = ctx.playback_clock.frame_duration.as_secs_f64();
            if gap < 0.0 {
                *ctx.video_pts_backtrack = ctx.video_pts_backtrack.saturating_add(1);
            }
            if expected > 0.0 {
                let jitter_ms = ((gap - expected).abs()) * 1000.0;
                *ctx.video_pts_jitter_abs_sum_ms += jitter_ms;
                if jitter_ms > *ctx.video_pts_jitter_max_ms {
                    *ctx.video_pts_jitter_max_ms = jitter_ms;
                }
            }
            if gap.is_finite() && gap > expected * 1.8 {
                emit_debug(
                    ctx.app,
                    "video_gap",
                    format!("detected frame pts gap={gap:.3}s expected~{expected:.3}s"),
                );
            }
        }
        *ctx.last_video_pts_seconds = Some(estimated_pts);
        *ctx.current_position_seconds = if ctx.duration_seconds > 0.0 {
            position_seconds.min(ctx.duration_seconds)
        } else {
            position_seconds
        };
        write_latest_stream_position(&ctx.app.state::<MediaState>(), *ctx.current_position_seconds)?;
        ctx.renderer.update_clock(
            *ctx.current_position_seconds,
            ctx.playback_clock.playback_rate(),
        );
        let Some(render_frame) = ctx
            .frame_pipeline
            .frame_to_renderer(ctx.app, &nv12_frame, estimated_pts)
        else {
            continue;
        };
        if !ctx
            .app
            .state::<MediaState>()
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        ctx.renderer.submit_frame(render_frame);
        ctx.frame_pipeline.record_frame_cost(frame_cost_start.elapsed());
        if let Some(render_fps) = ctx.fps_window.record_frame_and_compute() {
            emit_video_telemetry(ctx, pict_type, render_fps, estimated_pts, &nv12_frame);
        }
        if ctx.last_progress_emit.elapsed() >= Duration::from_millis(200) {
            update_playback_progress(
                ctx.app,
                ctx.stream_generation,
                *ctx.current_position_seconds,
                ctx.duration_seconds,
                false,
            )?;
            *ctx.last_progress_emit = Instant::now();
        }
    }
    Ok(())
}

fn emit_video_telemetry(
    ctx: &mut DrainFramesContext<'_>,
    pict_type: ffi::AVPictureType,
    render_fps: f64,
    estimated_pts: f64,
    nv12_frame: &frame::Video,
) {
    let perf_snapshot = ctx.frame_pipeline.take_perf_snapshot();
    let process_snapshot = ctx.process_metrics.sample();
    let renderer_metrics = ctx.renderer.metrics_snapshot();
    emit_debug(ctx.app, "video_fps", format!("render_fps={render_fps:.2}"));
    let audio_now = ctx.audio_clock.map(|clock| clock.now_seconds());
    let audio_drift = audio_now.map(|audio_seconds| estimated_pts - audio_seconds);
    emit_debug(
        ctx.app,
        "av_sync",
        format!(
            "a_minus_v={:.3}ms audio_clock={} video_pts={:.3}s queue_depth={} lead_target={:.3}ms",
            audio_drift.unwrap_or(0.0) * 1000.0,
            audio_now
                .map(|value| format!("{value:.3}s"))
                .unwrap_or_else(|| "n/a".to_string()),
            estimated_pts.max(0.0),
            renderer_metrics.queue_depth,
            ctx.audio_allowed_lead_seconds * 1000.0,
        ),
    );
    emit_debug(
        ctx.app,
        "video_pipeline",
        format!(
            "pts={:.3}s queue_depth={} clock={:.3}s rate={:.2} output={}x{} decode_avg={:.2}ms decode_max={:.2}ms decode_p50={:.2}ms decode_p95={:.2}ms decode_p99={:.2}ms samples={}",
            estimated_pts.max(0.0),
            renderer_metrics.queue_depth,
            *ctx.current_position_seconds,
            ctx.playback_clock.playback_rate(),
            ctx.output_width,
            ctx.output_height,
            perf_snapshot.as_ref().map(|value| value.avg_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.max_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.p50_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.p95_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.p99_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.samples).unwrap_or(0),
        ),
    );
    emit_debug(
        ctx.app,
        "decode_cost_quantiles",
        format!(
            "p50={:.3}ms p95={:.3}ms p99={:.3}ms avg={:.3}ms max={:.3}ms samples={}",
            perf_snapshot.as_ref().map(|value| value.p50_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.p95_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.p99_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.avg_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.max_ms).unwrap_or(0.0),
            perf_snapshot.as_ref().map(|value| value.samples).unwrap_or(0),
        ),
    );
    let ts_stats = take_video_timestamp_stats(ctx);
    let frame_type_stats = take_frame_type_stats(ctx);
    let decode_quantiles = perf_snapshot.as_ref().map(|value| MediaDecodeQuantileStats {
        sample_count: value.samples,
        avg_ms: value.avg_ms,
        max_ms: value.max_ms,
        p50_ms: value.p50_ms,
        p95_ms: value.p95_ms,
        p99_ms: value.p99_ms,
    });
    let integrity_snapshot = ctx.frame_pipeline.integrity_snapshot();
    let total_frame_drops = integrity_snapshot
        .dropped_hw_transfer
        .saturating_add(integrity_snapshot.dropped_scale)
        .saturating_add(integrity_snapshot.dropped_nv12_extract);
    let (process_cpu_percent, process_memory_mb) = process_snapshot.unwrap_or((0.0, 0.0));
    emit_telemetry_payloads(
        ctx.app,
        MediaTelemetryPayload {
            source_fps: 1.0 / ctx.playback_clock.frame_duration.as_secs_f64().max(1e-6),
            render_fps,
            queue_depth: renderer_metrics.queue_depth,
            audio_queue_depth_sources: ctx.audio_queue_depth_sources,
            clock_seconds: *ctx.current_position_seconds,
            current_video_pts_seconds: Some(estimated_pts.max(0.0)),
            current_audio_clock_seconds: audio_now,
            current_frame_type: Some(picture_type_label(pict_type).to_string()),
            current_frame_width: Some(nv12_frame.width()),
            current_frame_height: Some(nv12_frame.height()),
            playback_rate: Some(ctx.playback_clock.playback_rate()),
            network_read_bytes_per_second: ctx.network_read_bps,
            media_required_bytes_per_second: ctx.media_required_bps,
            network_sustain_ratio: match (ctx.network_read_bps, ctx.media_required_bps) {
                (Some(read_bps), Some(required_bps)) if required_bps > 0.0 => {
                    Some((read_bps / required_bps).max(0.0))
                }
                _ => None,
            },
            audio_drift_seconds: audio_now.map(|value| estimated_pts - value),
            video_pts_gap_seconds: ctx
                .last_video_pts_seconds
                .as_ref()
                .map(|previous_pts| (estimated_pts - previous_pts).max(0.0)),
            seek_settle_ms: None,
            decode_avg_frame_cost_ms: perf_snapshot.as_ref().map(|value| value.avg_ms),
            decode_max_frame_cost_ms: perf_snapshot.as_ref().map(|value| value.max_ms),
            decode_samples: perf_snapshot.as_ref().map(|value| value.samples),
            decode_quantiles,
            video_timestamps: ts_stats,
            frame_types: frame_type_stats,
            process_cpu_percent: Some(process_cpu_percent),
            process_memory_mb: Some(process_memory_mb),
            gpu_queue_depth: Some(renderer_metrics.queue_depth),
            gpu_queue_capacity: Some(renderer_metrics.queue_capacity),
            gpu_queue_utilization: Some(
                (renderer_metrics.queue_depth as f64)
                    / (renderer_metrics.queue_capacity.max(1) as f64),
            ),
            render_estimated_cost_ms: Some(renderer_metrics.last_render_cost_ms),
            render_present_lag_ms: Some(renderer_metrics.last_present_lag_ms),
            video_packet_soft_error_count: Some(*ctx.video_packet_soft_error_count),
            video_frame_drop_count: Some(total_frame_drops),
            video_hw_transfer_drop_count: Some(integrity_snapshot.dropped_hw_transfer),
            video_nv12_drop_count: Some(integrity_snapshot.dropped_nv12_extract),
            video_scale_drop_count: Some(integrity_snapshot.dropped_scale),
        },
    );
}

fn take_video_timestamp_stats(ctx: &mut DrainFramesContext<'_>) -> Option<MediaVideoTimestampStats> {
    let elapsed = ctx.video_ts_window_start.elapsed();
    if elapsed < Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
        return None;
    }
    let samples = (*ctx.video_ts_samples).max(1);
    let stats = MediaVideoTimestampStats {
        samples,
        pts_missing_ratio_percent: (*ctx.video_pts_missing as f64) * 100.0 / (samples as f64),
        pts_backtrack_count: *ctx.video_pts_backtrack,
        jitter_avg_ms: *ctx.video_pts_jitter_abs_sum_ms / (samples as f64),
        jitter_max_ms: *ctx.video_pts_jitter_max_ms,
    };
    emit_debug(
        ctx.app,
        "video_timestamps",
        format!(
            "samples={} pts_missing={:.2}% backtrack={} jitter_avg={:.3}ms jitter_max={:.3}ms",
            stats.samples,
            stats.pts_missing_ratio_percent,
            stats.pts_backtrack_count,
            stats.jitter_avg_ms,
            stats.jitter_max_ms
        ),
    );
    *ctx.video_ts_window_start = Instant::now();
    *ctx.video_ts_samples = 0;
    *ctx.video_pts_missing = 0;
    *ctx.video_pts_backtrack = 0;
    *ctx.video_pts_jitter_abs_sum_ms = 0.0;
    *ctx.video_pts_jitter_max_ms = 0.0;
    Some(stats)
}

fn take_frame_type_stats(ctx: &mut DrainFramesContext<'_>) -> Option<MediaFrameTypeStats> {
    let elapsed = ctx.video_frame_type_window_start.elapsed();
    if elapsed < Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
        return None;
    }
    let total = *ctx.video_frame_type_i
        + *ctx.video_frame_type_p
        + *ctx.video_frame_type_b
        + *ctx.video_frame_type_other;
    let stats = if total > 0 {
        let stats = MediaFrameTypeStats {
            sample_count: total,
            i_ratio_percent: (*ctx.video_frame_type_i as f64) * 100.0 / (total as f64),
            p_ratio_percent: (*ctx.video_frame_type_p as f64) * 100.0 / (total as f64),
            b_ratio_percent: (*ctx.video_frame_type_b as f64) * 100.0 / (total as f64),
            other_ratio_percent: (*ctx.video_frame_type_other as f64) * 100.0 / (total as f64),
        };
        emit_debug(
            ctx.app,
            "video_frame_types",
            format!(
                "I={:.1}% P={:.1}% B={:.1}% other={:.1}% samples={}",
                stats.i_ratio_percent,
                stats.p_ratio_percent,
                stats.b_ratio_percent,
                stats.other_ratio_percent,
                stats.sample_count
            ),
        );
        Some(stats)
    } else {
        None
    };
    *ctx.video_frame_type_window_start = Instant::now();
    *ctx.video_frame_type_i = 0;
    *ctx.video_frame_type_p = 0;
    *ctx.video_frame_type_b = 0;
    *ctx.video_frame_type_other = 0;
    stats
}

fn picture_type_label(pict_type: ffi::AVPictureType) -> &'static str {
    match pict_type {
        ffi::AVPictureType::AV_PICTURE_TYPE_I => "I",
        ffi::AVPictureType::AV_PICTURE_TYPE_P => "P",
        ffi::AVPictureType::AV_PICTURE_TYPE_B => "B",
        ffi::AVPictureType::AV_PICTURE_TYPE_S => "S",
        ffi::AVPictureType::AV_PICTURE_TYPE_SI => "SI",
        ffi::AVPictureType::AV_PICTURE_TYPE_SP => "SP",
        ffi::AVPictureType::AV_PICTURE_TYPE_BI => "BI",
        _ => "Other",
    }
}

pub(super) fn percentile_from_sorted(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let normalized = percentile.clamp(0.0, 100.0) / 100.0;
    let position = normalized * ((sorted.len() - 1) as f64);
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let weight = position - (lower as f64);
    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

pub const DECODE_LEAD_SLEEP_MS: u64 = 5;
pub const RENDER_BACKPRESSURE_SLEEP_MS: u64 = 2;
