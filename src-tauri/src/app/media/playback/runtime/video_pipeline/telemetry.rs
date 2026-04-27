use super::DrainFramesContext;
use crate::app::media::playback::events::{
    MediaDecodeQuantileStats, MediaFrameTypeStats, MediaTelemetryPayload, MediaVideoTimestampStats,
};
use crate::app::media::playback::runtime::{
    emit_debug, emit_telemetry_payloads, METRICS_EMIT_INTERVAL_MS,
};
use ffmpeg_next::ffi;
use ffmpeg_next::frame;
use std::time::{Duration, Instant};

pub(super) fn emit_video_telemetry(
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
            perf_snapshot
                .as_ref()
                .map(|value| value.p50_ms)
                .unwrap_or(0.0),
            perf_snapshot
                .as_ref()
                .map(|value| value.p95_ms)
                .unwrap_or(0.0),
            perf_snapshot
                .as_ref()
                .map(|value| value.p99_ms)
                .unwrap_or(0.0),
            perf_snapshot
                .as_ref()
                .map(|value| value.avg_ms)
                .unwrap_or(0.0),
            perf_snapshot
                .as_ref()
                .map(|value| value.max_ms)
                .unwrap_or(0.0),
            perf_snapshot
                .as_ref()
                .map(|value| value.samples)
                .unwrap_or(0),
        ),
    );
    let ts_stats = take_video_timestamp_stats(ctx);
    let frame_type_stats = take_frame_type_stats(ctx);
    let decode_quantiles = perf_snapshot
        .as_ref()
        .map(|value| MediaDecodeQuantileStats {
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
            source_fps: ctx.playback_clock.source_fps(),
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

fn take_video_timestamp_stats(
    ctx: &mut DrainFramesContext<'_>,
) -> Option<MediaVideoTimestampStats> {
    let elapsed = ctx.video_timestamp_metrics.window_start.elapsed();
    if elapsed < Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
        return None;
    }
    let samples = (*ctx.video_timestamp_metrics.samples).max(1);
    let stats = MediaVideoTimestampStats {
        samples,
        pts_missing_ratio_percent: (*ctx.video_timestamp_metrics.pts_missing as f64) * 100.0
            / (samples as f64),
        pts_backtrack_count: *ctx.video_timestamp_metrics.pts_backtrack,
        jitter_avg_ms: *ctx.video_timestamp_metrics.pts_jitter_abs_sum_ms / (samples as f64),
        jitter_max_ms: *ctx.video_timestamp_metrics.pts_jitter_max_ms,
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
    *ctx.video_timestamp_metrics.window_start = Instant::now();
    *ctx.video_timestamp_metrics.samples = 0;
    *ctx.video_timestamp_metrics.pts_missing = 0;
    *ctx.video_timestamp_metrics.pts_backtrack = 0;
    *ctx.video_timestamp_metrics.pts_jitter_abs_sum_ms = 0.0;
    *ctx.video_timestamp_metrics.pts_jitter_max_ms = 0.0;
    Some(stats)
}

fn take_frame_type_stats(ctx: &mut DrainFramesContext<'_>) -> Option<MediaFrameTypeStats> {
    let elapsed = ctx.video_frame_type_metrics.window_start.elapsed();
    if elapsed < Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
        return None;
    }
    let total = *ctx.video_frame_type_metrics.i_count
        + *ctx.video_frame_type_metrics.p_count
        + *ctx.video_frame_type_metrics.b_count
        + *ctx.video_frame_type_metrics.other_count;
    let stats = if total > 0 {
        let stats = MediaFrameTypeStats {
            sample_count: total,
            i_ratio_percent: (*ctx.video_frame_type_metrics.i_count as f64) * 100.0
                / (total as f64),
            p_ratio_percent: (*ctx.video_frame_type_metrics.p_count as f64) * 100.0
                / (total as f64),
            b_ratio_percent: (*ctx.video_frame_type_metrics.b_count as f64) * 100.0
                / (total as f64),
            other_ratio_percent: (*ctx.video_frame_type_metrics.other_count as f64) * 100.0
                / (total as f64),
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
    *ctx.video_frame_type_metrics.window_start = Instant::now();
    *ctx.video_frame_type_metrics.i_count = 0;
    *ctx.video_frame_type_metrics.p_count = 0;
    *ctx.video_frame_type_metrics.b_count = 0;
    *ctx.video_frame_type_metrics.other_count = 0;
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
