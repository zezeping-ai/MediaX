use crate::app::media::player::renderer::VideoFrame;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::software::scaling::flag::Flags;

pub fn ensure_scaler(
    scaler: &mut Option<ScalingContext>,
    src_format: format::pixel::Pixel,
    src_width: u32,
    src_height: u32,
    dst_format: format::pixel::Pixel,
    dst_width: u32,
    dst_height: u32,
    flags: Flags,
) -> Result<(), String> {
    let needs_new = match scaler {
        Some(existing) => {
            existing.input().format != src_format
                || existing.input().width != src_width
                || existing.input().height != src_height
                || existing.output().format != dst_format
                || existing.output().width != dst_width
                || existing.output().height != dst_height
        }
        None => true,
    };
    if !needs_new {
        return Ok(());
    }
    *scaler = Some(
        ScalingContext::get(
            src_format, src_width, src_height, dst_format, dst_width, dst_height, flags,
        )
        .map_err(|err| format!("scaler create failed: {err}"))?,
    );
    Ok(())
}

pub fn transfer_hw_frame_if_needed(decoded: &frame::Video) -> Result<frame::Video, String> {
    if !is_hardware_frame(decoded)? {
        return Ok(decoded.clone());
    }
    let mut sw_frame = frame::Video::empty();
    // SAFETY: Both frame pointers are owned AVFrame instances. `sw_frame` is empty output
    // buffer and `decoded` is a valid decoded frame from FFmpeg.
    let ret = unsafe { ffi::av_hwframe_transfer_data(sw_frame.as_mut_ptr(), decoded.as_ptr(), 0) };
    if ret < 0 {
        return Err(format!("hwframe transfer failed: {ret}"));
    }
    Ok(sw_frame)
}

fn is_hardware_frame(frame: &frame::Video) -> Result<bool, String> {
    let pix_fmt = frame.format().into();
    // SAFETY: Descriptor lookup is read-only for a valid pixel format enum.
    let desc = unsafe { ffi::av_pix_fmt_desc_get(pix_fmt) };
    if desc.is_null() {
        return Err("pixel format descriptor unavailable".to_string());
    }
    // SAFETY: `desc` is checked non-null above.
    let flags = unsafe { (*desc).flags };
    Ok((flags & ffi::AV_PIX_FMT_FLAG_HWACCEL as u64) != 0)
}

pub fn video_frame_to_nv12_planes(frame: &frame::Video, pts_seconds: Option<f64>) -> VideoFrame {
    let width = frame.width() as usize;
    let height = frame.height() as usize;

    let y_stride = frame.stride(0);
    let y_data = frame.data(0);
    let mut y_plane = Vec::with_capacity(width * height);
    for y in 0..height {
        let start = y * y_stride;
        let end = start + width;
        y_plane.extend_from_slice(&y_data[start..end]);
    }

    let uv_height = height / 2;
    let uv_row_bytes = width;
    let uv_stride = frame.stride(1);
    let uv_data = frame.data(1);
    let mut uv_plane = Vec::with_capacity(uv_row_bytes * uv_height);
    for y in 0..uv_height {
        let start = y * uv_stride;
        let end = start + uv_row_bytes;
        uv_plane.extend_from_slice(&uv_data[start..end]);
    }

    let (color_matrix, y_offset, y_scale, uv_offset, uv_scale) = color_conversion_params(frame);
    VideoFrame {
        pts_seconds: pts_seconds.unwrap_or(0.0),
        width: frame.width(),
        height: frame.height(),
        y_plane,
        uv_plane,
        color_matrix,
        y_offset,
        y_scale,
        uv_offset,
        uv_scale,
    }
}

fn color_conversion_params(frame: &frame::Video) -> ([[f32; 3]; 3], f32, f32, f32, f32) {
    use ffmpeg_next::color::{Range, Space};
    let matrix = match frame.color_space() {
        Space::BT2020NCL | Space::BT2020CL => [
            [1.0, 0.0, 1.4746],
            [1.0, -0.16455, -0.57135],
            [1.0, 1.8814, 0.0],
        ],
        Space::BT470BG | Space::SMPTE170M | Space::FCC | Space::SMPTE240M => [
            [1.0, 0.0, 1.402],
            [1.0, -0.344136, -0.714136],
            [1.0, 1.772, 0.0],
        ],
        _ => [
            [1.0, 0.0, 1.5748],
            [1.0, -0.1873, -0.4681],
            [1.0, 1.8556, 0.0],
        ],
    };
    match frame.color_range() {
        Range::JPEG => (matrix, 0.0, 1.0, 0.5, 1.0),
        _ => (matrix, 16.0 / 255.0, 255.0 / 219.0, 0.5, 255.0 / 224.0),
    }
}
