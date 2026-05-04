use crate::app::media::playback::render::renderer::{VideoFrame, VideoFramePlanes};
use ffmpeg_next::format;
use image::imageops::FilterType;

const COVER_MAX_EDGE: u32 = 1440;

pub(super) fn extract_cover_frame(
    input_ctx: &format::context::Input,
    stream_index: Option<usize>,
) -> Option<VideoFrame> {
    stream_index
        .and_then(|index| extract_cover_packet_bytes(input_ctx, index))
        .and_then(|bytes| cover_frame_from_image_bytes(&bytes).ok())
}

pub(super) fn extract_cover_packet_bytes(
    input_ctx: &format::context::Input,
    stream_index: usize,
) -> Option<Vec<u8>> {
    let stream = input_ctx
        .streams()
        .find(|value| value.index() == stream_index)?;
    // SAFETY: `stream` comes from the live format context and `attached_pic` is read-only.
    let packet = unsafe { &(*stream.as_ptr()).attached_pic };
    if packet.data.is_null() || packet.size <= 0 {
        return None;
    }
    // SAFETY: FFmpeg owns the packet buffer for the lifetime of the input context.
    let bytes = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
    Some(bytes.to_vec())
}

pub(crate) fn cover_frame_from_image_bytes(bytes: &[u8]) -> Result<VideoFrame, String> {
    let image =
        image::load_from_memory(bytes).map_err(|err| format!("decode cover art failed: {err}"))?;
    let rgb = image.to_rgb8();
    let (scaled_width, scaled_height) =
        fit_dimensions_within(rgb.width(), rgb.height(), COVER_MAX_EDGE);
    let rgb = if scaled_width != rgb.width() || scaled_height != rgb.height() {
        // Cover art is non-critical UI; prefer a simpler resizer to reduce CPU and avoid
        // hitting the heavier generic DynamicImage resize path on malformed attachments.
        image::imageops::resize(&rgb, scaled_width, scaled_height, FilterType::Triangle)
    } else {
        rgb
    };
    let mut width = rgb.width().max(2);
    let mut height = rgb.height().max(2);
    if width % 2 != 0 {
        width += 1;
    }
    if height % 2 != 0 {
        height += 1;
    }
    let resized = if width != rgb.width() || height != rgb.height() {
        image::imageops::resize(&rgb, width, height, FilterType::Lanczos3)
    } else {
        rgb
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    let mut y_plane = Vec::with_capacity(width_usize * height_usize);
    let mut uv_plane = Vec::with_capacity(width_usize * height_usize / 2);

    for y in 0..height {
        for x in 0..width {
            let pixel = resized.get_pixel(x, y);
            let (r, g, b) = (pixel[0] as f32, pixel[1] as f32, pixel[2] as f32);
            let luma = (0.299 * r + 0.587 * g + 0.114 * b)
                .round()
                .clamp(0.0, 255.0) as u8;
            y_plane.push(luma);
        }
    }

    for y in (0..height).step_by(2) {
        for x in (0..width).step_by(2) {
            let mut r_sum = 0.0;
            let mut g_sum = 0.0;
            let mut b_sum = 0.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let pixel = resized.get_pixel(x + dx, y + dy);
                    r_sum += pixel[0] as f32;
                    g_sum += pixel[1] as f32;
                    b_sum += pixel[2] as f32;
                }
            }
            let r = r_sum / 4.0;
            let g = g_sum / 4.0;
            let b = b_sum / 4.0;
            let u = (-0.168736 * r - 0.331264 * g + 0.5 * b + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            let v = (0.5 * r - 0.418688 * g - 0.081312 * b + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            uv_plane.push(u);
            uv_plane.push(v);
        }
    }

    Ok(VideoFrame {
        pts_seconds: 0.0,
        width,
        height,
        plane_strides: [width, width],
        planes: VideoFramePlanes::Nv12 { y_plane, uv_plane },
        color_matrix: [
            [1.0, 0.0, 1.402],
            [1.0, -0.344136, -0.714136],
            [1.0, 1.772, 0.0],
        ],
        y_offset: 0.0,
        y_scale: 1.0,
        uv_offset: 0.5,
        uv_scale: 1.0,
    })
}

fn fit_dimensions_within(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    if width == 0 || height == 0 || width <= max_edge && height <= max_edge {
        return (width.max(1), height.max(1));
    }
    if width >= height {
        let scaled_height = ((u64::from(height) * u64::from(max_edge)) / u64::from(width))
            .max(1)
            .min(u64::from(u32::MAX)) as u32;
        (max_edge, scaled_height)
    } else {
        let scaled_width = ((u64::from(width) * u64::from(max_edge)) / u64::from(height))
            .max(1)
            .min(u64::from(u32::MAX)) as u32;
        (scaled_width, max_edge)
    }
}
