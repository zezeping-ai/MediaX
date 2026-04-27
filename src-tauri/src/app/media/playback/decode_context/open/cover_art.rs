use crate::app::media::playback::render::renderer::VideoFrame;
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

fn extract_cover_packet_bytes(
    input_ctx: &format::context::Input,
    stream_index: usize,
) -> Option<Vec<u8>> {
    let stream = input_ctx.streams().find(|value| value.index() == stream_index)?;
    // SAFETY: `stream` comes from the live format context and `attached_pic` is read-only.
    let packet = unsafe { &(*stream.as_ptr()).attached_pic };
    if packet.data.is_null() || packet.size <= 0 {
        return None;
    }
    // SAFETY: FFmpeg owns the packet buffer for the lifetime of the input context.
    let bytes = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
    Some(bytes.to_vec())
}

fn cover_frame_from_image_bytes(bytes: &[u8]) -> Result<VideoFrame, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("decode cover art failed: {err}"))?;
    let resized = image.resize(COVER_MAX_EDGE, COVER_MAX_EDGE, FilterType::Lanczos3);
    let rgb = resized.to_rgb8();
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
            let luma = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
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
        y_plane,
        uv_plane,
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
