use crate::app::media::model::PreviewFrame;
use crate::app::media::playback::render::preview_config::{
    PREVIEW_FALLBACK_MIN_HEIGHT, PREVIEW_FALLBACK_MIN_WIDTH, PREVIEW_FALLBACK_SCALE_DEN,
    PREVIEW_FALLBACK_SCALE_NUM, PREVIEW_INITIAL_QUALITY, PREVIEW_MAX_BYTES,
    PREVIEW_MIN_QUALITY, PREVIEW_QUALITY_STEP, PREVIEW_TARGET_HEIGHT, PREVIEW_TARGET_WIDTH,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use ffmpeg_next::frame;
use image::codecs::jpeg::JpegEncoder;
use image::ColorType;

struct EncodedPreview {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

pub(super) fn create_preview_frame(
    frame: &frame::Video,
    position_seconds: f64,
) -> Result<PreviewFrame, String> {
    let encoded = encode_rgb_frame_to_small_jpeg(frame)?;
    Ok(PreviewFrame {
        mime_type: "image/jpeg".to_string(),
        data_base64: BASE64_STANDARD.encode(encoded.bytes),
        width: encoded.width,
        height: encoded.height,
        position_seconds: position_seconds.max(0.0),
    })
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
