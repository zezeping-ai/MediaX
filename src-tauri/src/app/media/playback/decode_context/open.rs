use super::hw_decode::configure_hw_decode;
use super::output_size::compute_output_size;
use super::types::{HwDecodeStatus, VideoDecodeContext};
use crate::app::media::model::{HardwareDecodeMode, PlaybackQualityMode};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::media::Type;

pub(crate) fn open_video_decode_context(
    source: &str,
    hw_mode: HardwareDecodeMode,
    quality_mode: PlaybackQualityMode,
    software_fallback_reason: Option<&str>,
) -> Result<VideoDecodeContext, String> {
    ffmpeg::init().map_err(|err| format!("ffmpeg init failed: {err}"))?;
    let input_ctx = format::input(source).map_err(|err| format!("open media failed: {err}"))?;
    let input_stream = input_ctx
        .streams()
        .best(Type::Video)
        .ok_or_else(|| "no video stream found".to_string())?;
    let video_stream_index = input_stream.index();
    let stream_time_base = input_stream.time_base();
    let stream_duration = input_stream.duration();
    let fps = input_stream.avg_frame_rate();
    let fps_value = if fps.denominator() != 0 {
        f64::from(fps.numerator()) / f64::from(fps.denominator())
    } else {
        0.0
    };
    let duration_seconds = if stream_duration > 0 {
        (stream_duration as f64) * f64::from(stream_time_base)
    } else {
        0.0
    };
    let mut codec_context = codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|err| format!("decoder context failed: {err}"))?;
    let hw_status = configure_hw_decode(&mut codec_context, hw_mode, software_fallback_reason)?;
    let decoder = match codec_context.decoder().video() {
        Ok(decoder) => decoder,
        Err(err) if hw_mode == HardwareDecodeMode::Auto && hw_status.active => {
            let fallback_reason =
                format!("auto fallback to software after decoder open failed: {err}");
            let mut software_context =
                codec::context::Context::from_parameters(input_stream.parameters())
                    .map_err(|ctx_err| format!("decoder context failed: {ctx_err}"))?;
            let software_status = configure_hw_decode(
                &mut software_context,
                HardwareDecodeMode::Off,
                Some(&fallback_reason),
            )?;
            let decoder = software_context
                .decoder()
                .video()
                .map_err(|decode_err| {
                    format!("video decoder create failed after fallback: {decode_err}")
                })?;
            return finalize_video_decode_context(
                input_ctx,
                video_stream_index,
                stream_time_base,
                fps_value,
                duration_seconds,
                quality_mode,
                decoder,
                software_status,
            );
        }
        Err(err) => {
            return Err(format!("video decoder create failed: {err}"));
        }
    };
    finalize_video_decode_context(
        input_ctx,
        video_stream_index,
        stream_time_base,
        fps_value,
        duration_seconds,
        quality_mode,
        decoder,
        hw_status,
    )
}

fn finalize_video_decode_context(
    input_ctx: format::context::Input,
    video_stream_index: usize,
    video_time_base: ffmpeg::Rational,
    fps_value: f64,
    duration_seconds: f64,
    quality_mode: PlaybackQualityMode,
    decoder: ffmpeg::decoder::Video,
    hw_status: HwDecodeStatus,
) -> Result<VideoDecodeContext, String> {
    let (output_width, output_height) =
        compute_output_size(decoder.width(), decoder.height(), quality_mode);
    Ok(VideoDecodeContext {
        input_ctx,
        video_stream_index,
        video_time_base,
        decoder,
        fps_value,
        duration_seconds,
        output_width,
        output_height,
        hw_decode_active: hw_status.active,
        hw_decode_backend: hw_status.backend,
        hw_decode_error: hw_status.error,
        hw_decode_decision: hw_status.decision,
    })
}
