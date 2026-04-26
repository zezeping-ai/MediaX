mod context;
mod decode;
mod encoder;

use self::context::{DecodePreviewFrameContext, PreviewFrameContext};
use self::decode::{decode_preview_frame_until, submit_preview_frame};
use crate::app::media::model::{HardwareDecodeMode, PlaybackQualityMode, PreviewFrame};
use crate::app::media::playback::decode_context::open_video_decode_context;
use crate::app::media::playback::render::preview_config::{
    PREVIEW_MAX_HEIGHT, PREVIEW_MAX_WIDTH, PREVIEW_MIN_HEIGHT, PREVIEW_MIN_WIDTH,
    PREVIEW_RENDER_TIMEOUT_MS, PREVIEW_TIMEOUT_MS,
};
use crate::app::media::playback::render::renderer::RendererState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::time::{Duration, Instant};

pub fn render_preview_frame_at<F>(
    renderer: &RendererState,
    source: &str,
    target_seconds: f64,
    should_abort: F,
) -> Result<(), String>
where
    F: Fn() -> bool,
{
    let mut video_ctx = open_video_decode_context(
        source,
        HardwareDecodeMode::Auto,
        PlaybackQualityMode::Source,
        None,
    )?;
    let mut scaler: Option<ScalingContext> = None;
    let input_ctx = &mut video_ctx.input_ctx;
    let video_stream_index = video_ctx.video_stream_index;
    let video_time_base = video_ctx.video_time_base;
    let decoder = &mut video_ctx.decoder;
    let clamped = target_seconds.max(0.0);
    let seek_applied = apply_preview_seek(input_ctx, decoder, clamped);

    let mut packet = Packet::empty();
    let deadline = Instant::now() + Duration::from_millis(PREVIEW_RENDER_TIMEOUT_MS);
    loop {
        if should_abort() {
            return Ok(());
        }
        match packet.read(input_ctx) {
            Ok(_) => {
                if packet.stream() != video_stream_index {
                    continue;
                }
                decoder
                    .send_packet(&packet)
                    .map_err(|err| format!("send packet failed: {err}"))?;
                let mut ctx = PreviewFrameContext {
                    renderer,
                    decoder,
                    scaler: &mut scaler,
                    output_width: video_ctx.output_width,
                    output_height: video_ctx.output_height,
                    video_time_base,
                    target_seconds: clamped,
                    seek_applied,
                    deadline,
                };
                if submit_preview_frame(&mut ctx, &should_abort)? {
                    return Ok(());
                }
            }
            Err(FfmpegError::Eof) => {
                decoder
                    .send_eof()
                    .map_err(|err| format!("send eof failed: {err}"))?;
                let mut ctx = PreviewFrameContext {
                    renderer,
                    decoder,
                    scaler: &mut scaler,
                    output_width: video_ctx.output_width,
                    output_height: video_ctx.output_height,
                    video_time_base,
                    target_seconds: clamped,
                    seek_applied,
                    deadline,
                };
                if submit_preview_frame(&mut ctx, &should_abort)? {
                    return Ok(());
                }
                break;
            }
            Err(_) => continue,
        }
    }

    Err("no frame available for preview".to_string())
}

pub fn generate_preview_frame<F>(
    source: &str,
    target_seconds: f64,
    max_width: u32,
    max_height: u32,
    should_abort: F,
) -> Result<Option<PreviewFrame>, String>
where
    F: Fn() -> bool,
{
    let mut video_ctx = open_video_decode_context(
        source,
        HardwareDecodeMode::Auto,
        PlaybackQualityMode::Source,
        None,
    )?;
    let input_ctx = &mut video_ctx.input_ctx;
    let video_stream_index = video_ctx.video_stream_index;
    let video_time_base = video_ctx.video_time_base;
    let decoder = &mut video_ctx.decoder;
    let target_w = max_width.clamp(PREVIEW_MIN_WIDTH, PREVIEW_MAX_WIDTH);
    let target_h = max_height.clamp(PREVIEW_MIN_HEIGHT, PREVIEW_MAX_HEIGHT);
    let mut rgb_scaler: Option<ScalingContext> = None;
    let clamped = target_seconds.max(0.0);
    let seek_applied = apply_preview_seek(input_ctx, decoder, clamped);

    let deadline = Instant::now() + Duration::from_millis(PREVIEW_TIMEOUT_MS);
    let mut packet = Packet::empty();
    loop {
        if should_abort() || Instant::now() >= deadline {
            return Ok(None);
        }
        match packet.read(input_ctx) {
            Ok(_) => {
                if packet.stream() != video_stream_index {
                    continue;
                }
                decoder
                    .send_packet(&packet)
                    .map_err(|err| format!("preview send packet failed: {err}"))?;
                let mut ctx = DecodePreviewFrameContext {
                    decoder,
                    scaler: &mut rgb_scaler,
                    output_width: target_w,
                    output_height: target_h,
                    video_time_base,
                    target_seconds: clamped,
                    seek_applied,
                    deadline,
                };
                if let Some(frame) = decode_preview_frame_until(&mut ctx, &should_abort)? {
                    return Ok(Some(frame));
                }
            }
            Err(FfmpegError::Eof) => {
                decoder
                    .send_eof()
                    .map_err(|err| format!("preview send eof failed: {err}"))?;
                let mut ctx = DecodePreviewFrameContext {
                    decoder,
                    scaler: &mut rgb_scaler,
                    output_width: target_w,
                    output_height: target_h,
                    video_time_base,
                    target_seconds: clamped,
                    seek_applied,
                    deadline,
                };
                if let Some(frame) = decode_preview_frame_until(&mut ctx, &should_abort)? {
                    return Ok(Some(frame));
                }
                return Ok(None);
            }
            Err(_) => continue,
        }
    }
}

fn apply_preview_seek(
    input_ctx: &mut ffmpeg::format::context::Input,
    decoder: &mut ffmpeg::decoder::Video,
    target_seconds: f64,
) -> bool {
    if target_seconds <= 0.0 {
        return false;
    }
    let target_timestamp = (target_seconds * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
    if input_ctx.seek(target_timestamp, ..).is_err() {
        return false;
    }
    decoder.flush();
    true
}
