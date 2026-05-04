use super::context::{DecodePreviewFrameContext, PreviewFrameContext};
use super::encoder::create_preview_frame;
use crate::app::media::model::PreviewFrame;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::video_frame::{
    can_bypass_scaler_for_renderer, detect_color_profile, ensure_scaler,
    preferred_scaled_format_for_renderer, transfer_or_prepare_decoder_frame_for_scale, ScalerSpec,
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
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        let hw_cpu = match transfer_or_prepare_decoder_frame_for_scale(&mut decoded) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let src: &frame::Video = hw_cpu.as_ref().unwrap_or(&decoded);
        ensure_scaler(
            ctx.scaler,
            ScalerSpec {
                src_format: src.format(),
                src_width: src.width(),
                src_height: src.height(),
                dst_format: preferred_scaled_format_for_renderer(src),
                dst_width: ctx.output_width,
                dst_height: ctx.output_height,
                flags: Flags::BILINEAR,
            },
        )?;
        let scaled_frame = if can_bypass_scaler_for_renderer(src, ctx.output_width, ctx.output_height)
        {
            if let Some(frame) = hw_cpu {
                frame
            } else {
                std::mem::replace(&mut decoded, frame::Video::empty())
            }
        } else {
            let mut out = frame::Video::empty();
            ctx.scaler
                .as_mut()
                .ok_or_else(|| "preview scaler missing after ensure_scaler".to_string())?
                .run(src, &mut out)
                .map_err(|err| format!("scale frame failed: {err}"))?;
            out
        };
        let color_profile = detect_color_profile(&scaled_frame);
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
            submit_preview_frame(ctx.renderer, scaled_frame, hinted_seconds, color_profile);
            return Ok(true);
        }

        if let Some(seconds) = hinted_seconds {
            if seconds + 0.04 >= ctx.target_seconds {
                submit_preview_frame(ctx.renderer, scaled_frame, hinted_seconds, color_profile);
                return Ok(true);
            }
        } else {
            submit_preview_frame(ctx.renderer, scaled_frame, hinted_seconds, color_profile);
            return Ok(true);
        }

        if Instant::now() >= ctx.deadline {
            submit_preview_frame(ctx.renderer, scaled_frame, hinted_seconds, color_profile);
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
            && hinted_seconds.is_some_and(|seconds| {
                seconds + 0.04 < ctx.target_seconds && Instant::now() < ctx.deadline
            })
        {
            continue;
        }
        let hw_cpu = match transfer_or_prepare_decoder_frame_for_scale(&mut decoded) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let src: &frame::Video = hw_cpu.as_ref().unwrap_or(&decoded);
        ensure_scaler(
            ctx.scaler,
            ScalerSpec {
                src_format: src.format(),
                src_width: src.width(),
                src_height: src.height(),
                dst_format: format::pixel::Pixel::RGB24,
                dst_width: ctx.output_width,
                dst_height: ctx.output_height,
                flags: Flags::BILINEAR,
            },
        )?;
        let rgb_frame = if can_bypass_scaler_for_renderer(src, ctx.output_width, ctx.output_height) {
            if let Some(frame) = hw_cpu {
                frame
            } else {
                std::mem::replace(&mut decoded, frame::Video::empty())
            }
        } else {
            let mut out = frame::Video::empty();
            ctx.scaler
                .as_mut()
                .ok_or_else(|| "preview scaler missing after ensure_scaler".to_string())?
                .run(src, &mut out)
                .map_err(|err| format!("preview scale rgb frame failed: {err}"))?;
            out
        };
        return Ok(Some(create_preview_frame(
            &rgb_frame,
            hinted_seconds.unwrap_or(ctx.target_seconds),
        )?));
    }
    Ok(None)
}
