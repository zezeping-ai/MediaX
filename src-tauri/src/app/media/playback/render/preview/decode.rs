use super::context::{DecodePreviewFrameContext, PreviewFrameContext};
use super::encoder::create_preview_frame;
use crate::app::media::model::PreviewFrame;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::video_frame::{
    detect_color_profile, ensure_scaler, transfer_hw_frame_if_needed, ScalerSpec,
};
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::flag::Flags;
use std::time::Instant;

pub(super) fn submit_preview_frame<F>(
    ctx: &mut PreviewFrameContext<'_>,
    should_abort: &F,
) -> Result<bool, String>
where
    F: Fn() -> bool,
{
    let mut decoded = frame::Video::empty();
    while ctx.decoder.receive_frame(&mut decoded).is_ok() {
        if should_abort() {
            return Ok(true);
        }
        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(_) => continue,
        };
        ensure_scaler(
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
        )?;
        let mut nv12_frame = frame::Video::empty();
        if let Some(scaler) = ctx.scaler.as_mut() {
            scaler
                .run(&frame_for_scale, &mut nv12_frame)
                .map_err(|err| format!("scale frame failed: {err}"))?;
        }
        let color_profile = detect_color_profile(&nv12_frame);
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        let submit_preview_frame = |renderer: &crate::app::media::playback::render::renderer::RendererState,
                                    frame: frame::Video,
                                    pts_seconds: Option<f64>,
                                    color_profile: crate::app::media::playback::render::video_frame::ColorProfile| {
            renderer.submit_decoded_frame(
                frame,
                pts_seconds.unwrap_or(0.0),
                color_profile.color_matrix,
                color_profile.y_offset,
                color_profile.y_scale,
                color_profile.uv_offset,
                color_profile.uv_scale,
            );
        };

        if ctx.seek_applied {
            submit_preview_frame(ctx.renderer, nv12_frame, hinted_seconds, color_profile);
            return Ok(true);
        }

        if let Some(seconds) = hinted_seconds {
            if seconds + 0.04 >= ctx.target_seconds {
                submit_preview_frame(ctx.renderer, nv12_frame, hinted_seconds, color_profile);
                return Ok(true);
            }
        } else {
            submit_preview_frame(ctx.renderer, nv12_frame, hinted_seconds, color_profile);
            return Ok(true);
        }

        if Instant::now() >= ctx.deadline {
            submit_preview_frame(ctx.renderer, nv12_frame, hinted_seconds, color_profile);
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn decode_preview_frame_until<F>(
    ctx: &mut DecodePreviewFrameContext<'_>,
    should_abort: &F,
) -> Result<Option<PreviewFrame>, String>
where
    F: Fn() -> bool,
{
    let mut decoded = frame::Video::empty();
    while ctx.decoder.receive_frame(&mut decoded).is_ok() {
        if should_abort() || Instant::now() >= ctx.deadline {
            return Ok(None);
        }
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        if !ctx.seek_applied
            && hinted_seconds
                .is_some_and(|seconds| seconds + 0.04 < ctx.target_seconds && Instant::now() < ctx.deadline)
        {
            continue;
        }
        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(_) => continue,
        };
        ensure_scaler(
            ctx.scaler,
            ScalerSpec {
                src_format: frame_for_scale.format(),
                src_width: frame_for_scale.width(),
                src_height: frame_for_scale.height(),
                dst_format: format::pixel::Pixel::RGB24,
                dst_width: ctx.output_width,
                dst_height: ctx.output_height,
                flags: Flags::BILINEAR,
            },
        )?;
        let mut rgb_frame = frame::Video::empty();
        if let Some(scaler) = ctx.scaler.as_mut() {
            scaler
                .run(&frame_for_scale, &mut rgb_frame)
                .map_err(|err| format!("preview scale rgb frame failed: {err}"))?;
        }
        return Ok(Some(create_preview_frame(
            &rgb_frame,
            hinted_seconds.unwrap_or(ctx.target_seconds),
        )?));
    }
    Ok(None)
}
