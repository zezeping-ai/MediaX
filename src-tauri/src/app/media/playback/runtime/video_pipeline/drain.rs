use super::constants::RENDER_BACKPRESSURE_SLEEP_MS;
use super::telemetry::emit_video_telemetry;
use super::DrainFramesContext;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::video_frame::{
    adaptive_upload_size_for_renderer, can_bypass_scaler_for_renderer, ensure_scaler,
    preferred_scaled_format_for_renderer, preferred_scaling_flags_for_renderer,
    transfer_hw_frame_if_needed, ScalerSpec,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::runtime::progress::{
    resolve_buffered_position_seconds, update_playback_progress,
};
use crate::app::media::playback::runtime::write_latest_stream_position;
use crate::app::media::state::MediaState;
use ffmpeg_next::ffi;
use ffmpeg_next::frame;
use std::time::{Duration, Instant};
use tauri::Manager;

pub(crate) fn drain_frames(ctx: &mut DrainFramesContext<'_>) -> Result<(), String> {
    let mut decoded = frame::Video::empty();
    let mut processed_frames = 0usize;
    loop {
        if ctx
            .max_frames_per_pass
            .is_some_and(|limit| processed_frames >= limit)
        {
            break;
        }
        let frame_cost_start = Instant::now();
        let receive_cost_start = Instant::now();
        let receive_result = ctx.decoder.receive_frame(&mut decoded);
        let receive_cost = receive_cost_start.elapsed();
        if receive_result.is_err() {
            break;
        }
        if !ctx
            .app
            .state::<MediaState>()
            .runtime
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let pict_type = unsafe { (*decoded.as_ptr()).pict_type };
        record_frame_type(ctx, pict_type);
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        *ctx.video_timestamp_metrics.samples =
            ctx.video_timestamp_metrics.samples.saturating_add(1);
        if decoded.pts().is_none() {
            *ctx.video_timestamp_metrics.pts_missing =
                ctx.video_timestamp_metrics.pts_missing.saturating_add(1);
        }
        let hinted_valid = hinted_seconds.filter(|value| value.is_finite() && *value >= 0.0);
        if should_skip_pre_seek_frame(ctx, hinted_valid) {
            continue;
        }
        let queue_wait_start = Instant::now();
        wait_for_render_capacity(ctx)?;
        let queue_wait_cost = queue_wait_start.elapsed();
        let Some((render_source_frame, hw_transfer_cost, scale_cost)) =
            scale_frame_for_render(ctx, &decoded)
        else {
            continue;
        };
        let estimated_pts = update_playback_position(ctx, hinted_seconds, hinted_valid);
        write_latest_stream_position(
            &ctx.app.state::<MediaState>(),
            *ctx.current_position_seconds,
        )?;
        ctx.renderer.update_clock(
            render_clock_anchor_seconds(ctx),
            ctx.playback_clock.playback_rate(),
        );
        let upload_prep_start = Instant::now();
        let Some((render_frame, color_profile_cost, frame_extract_cost)) =
            ctx.frame_pipeline
                .frame_to_renderer(ctx.app, render_source_frame, estimated_pts, None)
        else {
            continue;
        };
        let upload_prep_cost = upload_prep_start.elapsed();
        if !ctx
            .app
            .state::<MediaState>()
            .runtime
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        let render_frame_width = render_frame.frame.width();
        let render_frame_height = render_frame.frame.height();
        let render_frame_format = render_frame.frame.format();
        let submit_cost_start = Instant::now();
        ctx.renderer.submit_decoded_frame(
            render_frame.frame,
            render_frame.pts_seconds,
            render_frame.color_matrix,
            render_frame.y_offset,
            render_frame.y_scale,
            render_frame.uv_offset,
            render_frame.uv_scale,
        );
        let submit_cost = submit_cost_start.elapsed();
        ctx.frame_pipeline
            .record_frame_cost(frame_cost_start.elapsed());
        ctx.frame_pipeline.record_stage_costs(
            receive_cost,
            queue_wait_cost,
            hw_transfer_cost,
            scale_cost,
            color_profile_cost,
            frame_extract_cost,
            upload_prep_cost,
            submit_cost,
            frame_cost_start.elapsed(),
        );
        if let Some(render_fps) = ctx.fps_window.record_frame_and_compute() {
            let mut telemetry_frame = frame::Video::empty();
            telemetry_frame.set_width(render_frame_width);
            telemetry_frame.set_height(render_frame_height);
            telemetry_frame.set_format(render_frame_format);
            emit_video_telemetry(
                ctx,
                pict_type,
                render_fps,
                estimated_pts,
                &telemetry_frame,
            );
        }
        if ctx.last_progress_emit.elapsed() >= Duration::from_millis(200) {
            let buffered_position_seconds = resolve_buffered_position_seconds(
                ctx.input_ctx,
                ctx.duration_seconds,
                *ctx.current_position_seconds,
                ctx.is_network_source,
                ctx.is_realtime_source,
            );
            update_playback_progress(
                ctx.app,
                ctx.stream_generation,
                *ctx.current_position_seconds,
                ctx.duration_seconds,
                buffered_position_seconds,
                false,
            )?;
            *ctx.last_progress_emit = Instant::now();
        }
        processed_frames = processed_frames.saturating_add(1);
    }
    Ok(())
}

fn record_frame_type(ctx: &mut DrainFramesContext<'_>, pict_type: ffi::AVPictureType) {
    if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_I {
        *ctx.video_frame_type_metrics.i_count =
            ctx.video_frame_type_metrics.i_count.saturating_add(1);
    } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_P {
        *ctx.video_frame_type_metrics.p_count =
            ctx.video_frame_type_metrics.p_count.saturating_add(1);
    } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_B {
        *ctx.video_frame_type_metrics.b_count =
            ctx.video_frame_type_metrics.b_count.saturating_add(1);
    } else {
        *ctx.video_frame_type_metrics.other_count =
            ctx.video_frame_type_metrics.other_count.saturating_add(1);
    }
}

fn should_skip_pre_seek_frame(ctx: &mut DrainFramesContext<'_>, hinted_valid: Option<f64>) -> bool {
    if let (Some(target), Some(hint)) = (*ctx.active_seek_target_seconds, hinted_valid) {
        if hint + 0.03 < target {
            return true;
        }
        *ctx.active_seek_target_seconds = None;
    }
    false
}

fn wait_for_render_capacity(ctx: &DrainFramesContext<'_>) -> Result<(), String> {
    while !ctx.renderer.can_accept_frame() {
        ctx.renderer.wait_for_frame_slot(
            ctx.stop_flag,
            Duration::from_millis(RENDER_BACKPRESSURE_SLEEP_MS),
        )?;
    }
    Ok(())
}

fn scale_frame_for_render(
    ctx: &mut DrainFramesContext<'_>,
    decoded: &frame::Video,
) -> Option<(frame::Video, Duration, Duration)> {
    let hw_transfer_cost_start = Instant::now();
    let frame_for_scale = match transfer_hw_frame_if_needed(decoded) {
        Ok(frame) => frame,
        Err(err) => {
            ctx.frame_pipeline.on_hw_transfer_failed(ctx.app, &err);
            return None;
        }
    };
    let hw_transfer_cost = hw_transfer_cost_start.elapsed();
    let scale_cost_start = Instant::now();
    let (surface_width, surface_height) = ctx.renderer.surface_size();
    let render_target_width = if surface_width > 0 {
        surface_width
    } else {
        ctx.output_width
    };
    let render_target_height = if surface_height > 0 {
        surface_height
    } else {
        ctx.output_height
    };
    let (target_width, target_height) = adaptive_upload_size_for_renderer(
        frame_for_scale.width(),
        frame_for_scale.height(),
        render_target_width,
        render_target_height,
        ctx.playback_clock.source_fps(),
    );
    if can_bypass_scaler_for_renderer(&frame_for_scale, target_width, target_height) {
        return Some((frame_for_scale, hw_transfer_cost, scale_cost_start.elapsed()));
    }
    if let Err(err) = ensure_scaler(
        ctx.scaler,
        ScalerSpec {
            src_format: frame_for_scale.format(),
            src_width: frame_for_scale.width(),
            src_height: frame_for_scale.height(),
            dst_format: preferred_scaled_format_for_renderer(&frame_for_scale),
            dst_width: target_width,
            dst_height: target_height,
            flags: preferred_scaling_flags_for_renderer(
                frame_for_scale.width(),
                frame_for_scale.height(),
                target_width,
                target_height,
            ),
        },
    ) {
        ctx.frame_pipeline.on_scale_failed(ctx.app, &err);
        return None;
    }
    let mut scaled_frame = frame::Video::empty();
    if let Some(scaler) = ctx.scaler.as_mut() {
        if let Err(err) = scaler.run(&frame_for_scale, &mut scaled_frame) {
            ctx.frame_pipeline
                .on_scale_failed(ctx.app, &format!("scale frame failed: {err}"));
            return None;
        }
    }
    Some((scaled_frame, hw_transfer_cost, scale_cost_start.elapsed()))
}

fn update_playback_position(
    ctx: &mut DrainFramesContext<'_>,
    hinted_seconds: Option<f64>,
    hinted_valid: Option<f64>,
) -> f64 {
    let audio_now_seconds = ctx.audio_clock.map(|clock| clock.now_seconds());
    let position_seconds = ctx.playback_clock.tick(
        hinted_seconds,
        audio_now_seconds,
        ctx.audio_queue_depth_sources,
        ctx.audio_allowed_lead_seconds,
    );
    let estimated_pts = hinted_valid.unwrap_or_else(|| {
        if let Some(previous_pts) = *ctx.last_video_pts_seconds {
            previous_pts + ctx.playback_clock.frame_duration_seconds()
        } else {
            position_seconds.max(0.0)
        }
    });
    if let Some(previous_pts) = *ctx.last_video_pts_seconds {
        let gap = estimated_pts - previous_pts;
        *ctx.video_timestamp_metrics.last_gap_seconds = Some(gap);
        let expected = ctx.playback_clock.frame_duration_seconds();
        if gap < 0.0 {
            *ctx.video_timestamp_metrics.pts_backtrack =
                ctx.video_timestamp_metrics.pts_backtrack.saturating_add(1);
        }
        if expected > 0.0 {
            let jitter_ms = ((gap - expected).abs()) * 1000.0;
            *ctx.video_timestamp_metrics.pts_jitter_abs_sum_ms += jitter_ms;
            if jitter_ms > *ctx.video_timestamp_metrics.pts_jitter_max_ms {
                *ctx.video_timestamp_metrics.pts_jitter_max_ms = jitter_ms;
            }
        }
        if gap.is_finite() && gap > expected * 1.8 {
            emit_debug(
                ctx.app,
                "video_gap",
                format!("detected frame pts gap={gap:.3}s expected~{expected:.3}s"),
            );
        }
    } else {
        *ctx.video_timestamp_metrics.last_gap_seconds = None;
    }
    *ctx.last_video_pts_seconds = Some(estimated_pts);
    *ctx.current_position_seconds = if ctx.duration_seconds > 0.0 {
        position_seconds.min(ctx.duration_seconds)
    } else {
        position_seconds
    };
    estimated_pts
}

fn render_clock_anchor_seconds(ctx: &DrainFramesContext<'_>) -> f64 {
    // Keep decode/progress advancement separate from the render presentation clock.
    // With audio present, rebasing the renderer to the decode thread's newest position can
    // collapse queued future frames into "already due", which hurts smoothness on heavier GOPs.
    ctx.audio_clock
        .map(|clock| {
            let renderer_audio_lead_seconds = if ctx.is_realtime_source {
                0.0
            } else {
                (ctx.audio_allowed_lead_seconds * 0.5).min(0.006)
            };
            clock.now_seconds() + renderer_audio_lead_seconds
        })
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or_else(|| (*ctx.current_position_seconds).max(0.0))
}
