use super::DrainFramesContext;
use crate::app::media::playback::runtime::video_pipeline::frame_pipeline::VideoStagePerfSnapshot;
use crate::app::media::playback::events::{
    MediaDecodeQuantileStats, MediaFrameTypeStats, MediaTelemetryPayload,
    MediaVideoStageCostStats, MediaVideoTimestampStats,
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
    let stage_perf_snapshot = ctx.frame_pipeline.take_stage_perf_snapshot();
    let process_snapshot = ctx.process_metrics.sample();
    let renderer_metrics = ctx.renderer.metrics_snapshot();
    let requested_playback_rate = ctx.playback_clock.requested_playback_rate();
    let effective_playback_rate = ctx.playback_clock.playback_rate();
    let rate_limited_reason = ctx.playback_clock.playback_rate_limited_reason();
    let presented_video_pts = renderer_metrics.last_presented_pts_seconds;
    let effective_display_video_pts = renderer_metrics.effective_display_pts_seconds;
    let submitted_video_pts = renderer_metrics.last_submitted_pts_seconds;
    let audio_now = ctx.audio_clock.map(|clock| clock.now_seconds());
    let sync_video_pts = effective_display_video_pts
        .or(presented_video_pts)
        .unwrap_or_else(|| estimated_pts.max(0.0));
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
    let video_stage_costs = stage_perf_snapshot.as_ref().map(|value| MediaVideoStageCostStats {
        sample_count: value.sample_count,
        receive_avg_ms: value.receive_avg_ms,
        receive_max_ms: value.receive_max_ms,
        queue_wait_avg_ms: value.queue_wait_avg_ms,
        queue_wait_max_ms: value.queue_wait_max_ms,
        hw_transfer_avg_ms: value.hw_transfer_avg_ms,
        hw_transfer_max_ms: value.hw_transfer_max_ms,
        scale_avg_ms: value.scale_avg_ms,
        scale_max_ms: value.scale_max_ms,
        color_profile_avg_ms: value.color_profile_avg_ms,
        color_profile_max_ms: value.color_profile_max_ms,
        frame_extract_avg_ms: value.frame_extract_avg_ms,
        frame_extract_max_ms: value.frame_extract_max_ms,
        upload_prep_avg_ms: value.upload_prep_avg_ms,
        upload_prep_max_ms: value.upload_prep_max_ms,
        submit_avg_ms: value.submit_avg_ms,
        submit_max_ms: value.submit_max_ms,
        total_avg_ms: value.total_avg_ms,
        total_max_ms: value.total_max_ms,
    });
    let integrity_snapshot = ctx.frame_pipeline.integrity_snapshot();
    let total_frame_drops = integrity_snapshot
        .dropped_hw_transfer
        .saturating_add(integrity_snapshot.dropped_scale)
        .saturating_add(integrity_snapshot.dropped_nv12_extract);
    let (process_cpu_percent, process_memory_mb) = process_snapshot.unwrap_or((0.0, 0.0));
    emit_renderer_efficiency_debug(ctx, &renderer_metrics);
    emit_renderer_starved_debug(ctx, &renderer_metrics, stage_perf_snapshot.as_ref());
    emit_av_sync_debug(ctx, &renderer_metrics, audio_now, sync_video_pts);
    emit_telemetry_payloads(
        ctx.app,
        MediaTelemetryPayload {
            source_fps: ctx.playback_clock.source_fps(),
            render_fps,
            queue_depth: renderer_metrics.queue_depth,
            audio_queue_depth_sources: ctx.audio_queue_depth_sources,
            clock_seconds: *ctx.current_position_seconds,
            current_video_pts_seconds: Some(sync_video_pts),
            current_effective_display_video_pts_seconds: effective_display_video_pts,
            current_presented_video_pts_seconds: presented_video_pts,
            current_submitted_video_pts_seconds: submitted_video_pts,
            current_audio_clock_seconds: audio_now,
            current_frame_type: Some(picture_type_label(pict_type).to_string()),
            current_frame_width: Some(nv12_frame.width()),
            current_frame_height: Some(nv12_frame.height()),
            playback_rate: Some(effective_playback_rate),
            requested_playback_rate: Some(requested_playback_rate),
            effective_playback_rate: Some(effective_playback_rate),
            playback_rate_limited_reason: rate_limited_reason,
            network_read_bytes_per_second: ctx.network_read_bps,
            media_required_bytes_per_second: ctx.media_required_bps,
            network_sustain_ratio: match (ctx.network_read_bps, ctx.media_required_bps) {
                (Some(read_bps), Some(required_bps)) if required_bps > 0.0 => {
                    Some((read_bps / required_bps).max(0.0))
                }
                _ => None,
            },
            audio_drift_seconds: audio_now.map(|value| sync_video_pts - value),
            video_pts_gap_seconds: *ctx.video_timestamp_metrics.last_gap_seconds,
            seek_settle_ms: None,
            decode_avg_frame_cost_ms: perf_snapshot.as_ref().map(|value| value.avg_ms),
            decode_max_frame_cost_ms: perf_snapshot.as_ref().map(|value| value.max_ms),
            decode_samples: perf_snapshot.as_ref().map(|value| value.samples),
            decode_quantiles,
            video_stage_costs,
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
            render_loop_wakeups: Some(renderer_metrics.render_loop_wakeups),
            render_attempts: Some(renderer_metrics.render_attempts),
            render_presents: Some(renderer_metrics.render_presents),
            render_uploads: Some(renderer_metrics.render_uploads),
            video_submit_lead_ms: Some(renderer_metrics.submit_lead_ms),
            video_packet_soft_error_count: Some(*ctx.video_packet_soft_error_count),
            video_frame_drop_count: Some(total_frame_drops),
            video_hw_transfer_drop_count: Some(integrity_snapshot.dropped_hw_transfer),
            video_nv12_drop_count: Some(integrity_snapshot.dropped_nv12_extract),
            video_scale_drop_count: Some(integrity_snapshot.dropped_scale),
        },
    );
}

fn emit_renderer_efficiency_debug(
    ctx: &DrainFramesContext<'_>,
    renderer_metrics: &crate::app::media::playback::render::renderer::RendererMetricsSnapshot,
) {
    emit_debug(
        ctx.app,
        "renderer_efficiency",
        format!(
            "wakeups={} attempts={} presents={} uploads={} queue={}/{} head_pts={:.3?} tail_pts={:.3?} queued_span_ms={:.2?} lag_ms={:.2} render_ms={:.2} submit_lead_ms={:.2}",
            renderer_metrics.render_loop_wakeups,
            renderer_metrics.render_attempts,
            renderer_metrics.render_presents,
            renderer_metrics.render_uploads,
            renderer_metrics.queue_depth,
            renderer_metrics.queue_capacity,
            renderer_metrics.queued_head_pts_seconds,
            renderer_metrics.queued_tail_pts_seconds,
            match (
                renderer_metrics.queued_head_pts_seconds,
                renderer_metrics.queued_tail_pts_seconds,
            ) {
                (Some(head), Some(tail)) if tail >= head => Some((tail - head) * 1000.0),
                _ => None,
            },
            renderer_metrics.last_present_lag_ms,
            renderer_metrics.last_render_cost_ms,
            renderer_metrics.submit_lead_ms,
        ),
    );
}

fn emit_renderer_starved_debug(
    ctx: &DrainFramesContext<'_>,
    renderer_metrics: &crate::app::media::playback::render::renderer::RendererMetricsSnapshot,
    stage_perf_snapshot: Option<&VideoStagePerfSnapshot>,
) {
    let queue_depth = renderer_metrics.queue_depth;
    let queue_capacity = renderer_metrics.queue_capacity.max(1);
    let stage_summary = stage_perf_snapshot.map_or_else(
        || "stages=n/a".to_string(),
        |stats| {
            format!(
                "receive_avg_ms={:.2} queue_wait_avg_ms={:.2} hw_transfer_avg_ms={:.2} scale_avg_ms={:.2} upload_prep_avg_ms={:.2} submit_avg_ms={:.2} total_avg_ms={:.2}",
                stats.receive_avg_ms,
                stats.queue_wait_avg_ms,
                stats.hw_transfer_avg_ms,
                stats.scale_avg_ms,
                stats.upload_prep_avg_ms,
                stats.submit_avg_ms,
                stats.total_avg_ms,
            )
        },
    );
    let queue_empty = queue_depth == 0;
    let low_submit_lead = renderer_metrics.submit_lead_ms <= 20.0;
    let present_lagged = renderer_metrics.last_present_lag_ms >= 8.0;
    if queue_empty || low_submit_lead || present_lagged {
        emit_debug(
            ctx.app,
            "renderer_starved",
            format!(
                "queue={}/{} lag_ms={:.2} submit_lead_ms={:.2} audio_queue_depth={:?} network_read_bps={:?} media_required_bps={:?} {}",
                queue_depth,
                queue_capacity,
                renderer_metrics.last_present_lag_ms,
                renderer_metrics.submit_lead_ms,
                ctx.audio_queue_depth_sources,
                ctx.network_read_bps.map(|value| value.round() as i64),
                ctx.media_required_bps.map(|value| value.round() as i64),
                stage_summary,
            ),
        );
        return;
    }
    let queue_backpressured = queue_depth + 1 >= queue_capacity;
    let high_queue_wait = stage_perf_snapshot
        .map(|stats| stats.queue_wait_avg_ms >= 20.0)
        .unwrap_or(false);
    if !queue_backpressured || !high_queue_wait {
        return;
    }
    emit_debug(
        ctx.app,
        "renderer_backpressure",
        format!(
            "queue={}/{} lag_ms={:.2} submit_lead_ms={:.2} audio_queue_depth={:?} network_read_bps={:?} media_required_bps={:?} {}",
            queue_depth,
            queue_capacity,
            renderer_metrics.last_present_lag_ms,
            renderer_metrics.submit_lead_ms,
            ctx.audio_queue_depth_sources,
            ctx.network_read_bps.map(|value| value.round() as i64),
            ctx.media_required_bps.map(|value| value.round() as i64),
            stage_summary,
        ),
    );
}

fn emit_av_sync_debug(
    ctx: &DrainFramesContext<'_>,
    renderer_metrics: &crate::app::media::playback::render::renderer::RendererMetricsSnapshot,
    audio_now: Option<f64>,
    sync_video_pts: f64,
) {
    let Some(audio_seconds) = audio_now.filter(|value| value.is_finite() && *value >= 0.0) else {
        return;
    };
    let drift_ms = (sync_video_pts - audio_seconds) * 1000.0;
    let presented_drift_ms = renderer_metrics
        .last_presented_pts_seconds
        .filter(|value| value.is_finite())
        .map(|video_pts| (video_pts - audio_seconds) * 1000.0);
    emit_debug(
        ctx.app,
        "av_sync",
        format!(
            "drift_ms={:.2} effective_display_pts_ms={:.2?} presented_drift_ms={:.2?} audio_queue_depth={:?} audio_queued_ms={:.2?} queue={}/{} lag_ms={:.2} submit_lead_ms={:.2}",
            drift_ms,
            renderer_metrics
                .effective_display_pts_seconds
                .map(|video_pts| (video_pts - audio_seconds) * 1000.0),
            presented_drift_ms,
            ctx.audio_queue_depth_sources,
            ctx.audio_queued_seconds.map(|value| value * 1000.0),
            renderer_metrics.queue_depth,
            renderer_metrics.queue_capacity,
            renderer_metrics.last_present_lag_ms,
            renderer_metrics.submit_lead_ms,
        ),
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
    *ctx.video_timestamp_metrics.window_start = Instant::now();
    *ctx.video_timestamp_metrics.samples = 0;
    *ctx.video_timestamp_metrics.pts_missing = 0;
    *ctx.video_timestamp_metrics.pts_backtrack = 0;
    *ctx.video_timestamp_metrics.pts_jitter_abs_sum_ms = 0.0;
    *ctx.video_timestamp_metrics.pts_jitter_max_ms = 0.0;
    *ctx.video_timestamp_metrics.last_gap_seconds = None;
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
