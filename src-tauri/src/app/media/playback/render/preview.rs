use crate::app::media::model::{HardwareDecodeMode, PlaybackQualityMode, PreviewFrame};
use crate::app::media::playback::decode_context::open_video_decode_context;
use crate::app::media::playback::render::preview_config::{
    PREVIEW_FALLBACK_MIN_HEIGHT, PREVIEW_FALLBACK_MIN_WIDTH, PREVIEW_FALLBACK_SCALE_DEN,
    PREVIEW_FALLBACK_SCALE_NUM, PREVIEW_INITIAL_QUALITY, PREVIEW_MAX_BYTES, PREVIEW_MAX_HEIGHT,
    PREVIEW_MAX_WIDTH, PREVIEW_MIN_HEIGHT, PREVIEW_MIN_QUALITY, PREVIEW_MIN_WIDTH,
    PREVIEW_QUALITY_STEP, PREVIEW_RENDER_TIMEOUT_MS, PREVIEW_TARGET_HEIGHT, PREVIEW_TARGET_WIDTH,
    PREVIEW_TIMEOUT_MS,
};
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::render::video_frame::{
    detect_color_profile, ensure_scaler, transfer_hw_frame_if_needed,
    video_frame_to_nv12_planes_from_yuv420p,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use image::codecs::jpeg::JpegEncoder;
use image::ColorType;
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
    )?;
    let mut scaler: Option<ScalingContext> = None;
    let input_ctx = &mut video_ctx.input_ctx;
    let video_stream_index = video_ctx.video_stream_index;
    let video_time_base = video_ctx.video_time_base;
    let decoder = &mut video_ctx.decoder;

    let clamped = target_seconds.max(0.0);
    let mut seek_applied = false;
    if clamped > 0.0 {
        let ts = (clamped * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
        if input_ctx.seek(ts, ..).is_ok() {
            decoder.flush();
            seek_applied = true;
        }
    }

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
    )?;
    let input_ctx = &mut video_ctx.input_ctx;
    let video_stream_index = video_ctx.video_stream_index;
    let video_time_base = video_ctx.video_time_base;
    let decoder = &mut video_ctx.decoder;

    let target_w = max_width.clamp(PREVIEW_MIN_WIDTH, PREVIEW_MAX_WIDTH);
    let target_h = max_height.clamp(PREVIEW_MIN_HEIGHT, PREVIEW_MAX_HEIGHT);
    let mut rgb_scaler: Option<ScalingContext> = None;

    let clamped = target_seconds.max(0.0);
    let mut seek_applied = false;
    if clamped > 0.0 {
        let ts = (clamped * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
        if input_ctx.seek(ts, ..).is_ok() {
            decoder.flush();
            seek_applied = true;
        }
    }

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

fn submit_preview_frame<F>(
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
            crate::app::media::playback::render::video_frame::ScalerSpec {
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
        let frame = match video_frame_to_nv12_planes_from_yuv420p(
            &nv12_frame,
            None,
            Some(detect_color_profile(&nv12_frame)),
        ) {
            Ok(frame) => frame,
            Err(_) => continue,
        };

        if ctx.seek_applied {
            ctx.renderer.submit_frame(frame);
            return Ok(true);
        }

        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);

        if let Some(seconds) = hinted_seconds {
            if seconds + 0.04 >= ctx.target_seconds {
                ctx.renderer.submit_frame(frame);
                return Ok(true);
            }
        } else {
            ctx.renderer.submit_frame(frame);
            return Ok(true);
        }

        if Instant::now() >= ctx.deadline {
            ctx.renderer.submit_frame(frame);
            return Ok(true);
        }
    }
    Ok(false)
}

fn decode_preview_frame_until<F>(
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
        if !ctx.seek_applied {
            if let Some(seconds) = hinted_seconds {
                if seconds + 0.04 < ctx.target_seconds && Instant::now() < ctx.deadline {
                    continue;
                }
            }
        }
        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(_) => continue,
        };
        ensure_scaler(
            ctx.scaler,
            crate::app::media::playback::render::video_frame::ScalerSpec {
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
        let encoded = encode_rgb_frame_to_small_jpeg(&rgb_frame)?;
        return Ok(Some(PreviewFrame {
            mime_type: "image/jpeg".to_string(),
            data_base64: BASE64_STANDARD.encode(encoded.bytes),
            width: encoded.width,
            height: encoded.height,
            position_seconds: hinted_seconds.unwrap_or(ctx.target_seconds).max(0.0),
        }));
    }
    Ok(None)
}

struct PreviewFrameContext<'a> {
    renderer: &'a RendererState,
    decoder: &'a mut ffmpeg::decoder::Video,
    scaler: &'a mut Option<ScalingContext>,
    output_width: u32,
    output_height: u32,
    video_time_base: ffmpeg::Rational,
    target_seconds: f64,
    seek_applied: bool,
    deadline: Instant,
}

struct DecodePreviewFrameContext<'a> {
    decoder: &'a mut ffmpeg::decoder::Video,
    scaler: &'a mut Option<ScalingContext>,
    output_width: u32,
    output_height: u32,
    video_time_base: ffmpeg::Rational,
    target_seconds: f64,
    seek_applied: bool,
    deadline: Instant,
}

struct EncodedPreview {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

fn encode_rgb_frame_to_small_jpeg(frame: &frame::Video) -> Result<EncodedPreview, String> {
    let src_width = frame.width().max(1);
    let src_height = frame.height().max(1);
    let target_width = src_width.clamp(1, PREVIEW_TARGET_WIDTH);
    let target_height = src_height.clamp(1, PREVIEW_TARGET_HEIGHT);

    let mut working = frame_to_packed_rgb(frame)?;
    let mut width = src_width;
    let mut height = src_height;
    if width != target_width || height != target_height {
        let resized = image::imageops::resize(
            &image::RgbImage::from_raw(width, height, working)
                .ok_or_else(|| "preview rgb buffer size invalid".to_string())?,
            target_width,
            target_height,
            image::imageops::FilterType::Triangle,
        );
        working = resized.into_raw();
        width = target_width;
        height = target_height;
    }

    let mut quality = PREVIEW_INITIAL_QUALITY;
    loop {
        let mut encoded = Vec::new();
        let mut encoder = JpegEncoder::new_with_quality(&mut encoded, quality);
        encoder
            .encode(&working, width, height, ColorType::Rgb8.into())
            .map_err(|err| format!("preview jpeg encode failed: {err}"))?;
        if encoded.len() <= PREVIEW_MAX_BYTES
            || (width <= PREVIEW_FALLBACK_MIN_WIDTH
                && height <= PREVIEW_FALLBACK_MIN_HEIGHT
                && quality <= PREVIEW_MIN_QUALITY)
        {
            return Ok(EncodedPreview {
                bytes: encoded,
                width,
                height,
            });
        }
        if quality > PREVIEW_MIN_QUALITY {
            quality = quality.saturating_sub(PREVIEW_QUALITY_STEP);
            continue;
        }
        let next_w = (width.saturating_mul(PREVIEW_FALLBACK_SCALE_NUM)
            / PREVIEW_FALLBACK_SCALE_DEN)
            .max(PREVIEW_FALLBACK_MIN_WIDTH);
        let next_h = (height.saturating_mul(PREVIEW_FALLBACK_SCALE_NUM)
            / PREVIEW_FALLBACK_SCALE_DEN)
            .max(PREVIEW_FALLBACK_MIN_HEIGHT);
        if next_w == width && next_h == height {
            return Ok(EncodedPreview {
                bytes: encoded,
                width,
                height,
            });
        }
        let resized = image::imageops::resize(
            &image::RgbImage::from_raw(width, height, working)
                .ok_or_else(|| "preview rgb buffer size invalid".to_string())?,
            next_w,
            next_h,
            image::imageops::FilterType::Triangle,
        );
        width = next_w;
        height = next_h;
        working = resized.into_raw();
    }
}

fn frame_to_packed_rgb(frame: &frame::Video) -> Result<Vec<u8>, String> {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    let stride = frame.stride(0);
    let data = frame.data(0);
    let row_bytes = width * 3;
    let mut packed = Vec::with_capacity(row_bytes * height);
    for y in 0..height {
        let start = y * stride;
        let end = start + row_bytes;
        if end > data.len() {
            return Err("preview rgb frame stride out of bounds".to_string());
        }
        packed.extend_from_slice(&data[start..end]);
    }
    Ok(packed)
}
