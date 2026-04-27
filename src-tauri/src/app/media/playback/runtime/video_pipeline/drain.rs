use super::constants::RENDER_BACKPRESSURE_SLEEP_MS;
use super::telemetry::emit_video_telemetry;
use super::DrainFramesContext;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::video_frame::{
    ensure_scaler, transfer_hw_frame_if_needed, ScalerSpec,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::playback::runtime::write_latest_stream_position;
use crate::app::media::state::MediaState;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::flag::Flags;
use std::time::{Duration, Instant};
use tauri::Manager;

pub(crate) fn drain_frames(ctx: &mut DrainFramesContext<'_>) -> Result<(), String> {
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
        wait_for_render_capacity(ctx)?;
        let Some(nv12_frame) = scale_frame_for_render(ctx, &decoded) else {
            continue;
        };
        let estimated_pts = update_playback_position(ctx, hinted_seconds, hinted_valid);
        write_latest_stream_position(
            &ctx.app.state::<MediaState>(),
            *ctx.current_position_seconds,
        )?;
        ctx.renderer.update_clock(
            *ctx.current_position_seconds,
            ctx.playback_clock.playback_rate(),
        );
        let Some(render_frame) =
            ctx.frame_pipeline
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
        ctx.frame_pipeline
            .record_frame_cost(frame_cost_start.elapsed());
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
        if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(RENDER_BACKPRESSURE_SLEEP_MS));
    }
    Ok(())
}

fn scale_frame_for_render(
    ctx: &mut DrainFramesContext<'_>,
    decoded: &frame::Video,
) -> Option<frame::Video> {
    let frame_for_scale = match transfer_hw_frame_if_needed(decoded) {
        Ok(frame) => frame,
        Err(err) => {
            ctx.frame_pipeline.on_hw_transfer_failed(ctx.app, &err);
            return None;
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
        return None;
    }
    let mut nv12_frame = frame::Video::empty();
    if let Some(scaler) = ctx.scaler.as_mut() {
        if let Err(err) = scaler.run(&frame_for_scale, &mut nv12_frame) {
            ctx.frame_pipeline
                .on_scale_failed(ctx.app, &format!("scale frame failed: {err}"));
            return None;
        }
    }
    let _ = ctx
        .frame_pipeline
        .resolve_color_profile(ctx.app, &nv12_frame);
    Some(nv12_frame)
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
    }
    *ctx.last_video_pts_seconds = Some(estimated_pts);
    *ctx.current_position_seconds = if ctx.duration_seconds > 0.0 {
        position_seconds.min(ctx.duration_seconds)
    } else {
        position_seconds
    };
    estimated_pts
}
