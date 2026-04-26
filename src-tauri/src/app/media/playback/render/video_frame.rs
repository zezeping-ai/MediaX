use crate::app::media::playback::render::renderer::VideoFrame;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::software::scaling::flag::Flags;

#[derive(Clone, Copy)]
pub struct ColorProfile {
    pub color_matrix: [[f32; 3]; 3],
    pub y_offset: f32,
    pub y_scale: f32,
    pub uv_offset: f32,
    pub uv_scale: f32,
}

pub fn ensure_scaler(scaler: &mut Option<ScalingContext>, spec: ScalerSpec) -> Result<(), String> {
    let needs_new = match scaler {
        Some(existing) => {
            existing.input().format != spec.src_format
                || existing.input().width != spec.src_width
                || existing.input().height != spec.src_height
                || existing.output().format != spec.dst_format
                || existing.output().width != spec.dst_width
                || existing.output().height != spec.dst_height
        }
        None => true,
    };
    if !needs_new {
        return Ok(());
    }
    *scaler = Some(
        ScalingContext::get(
            spec.src_format,
            spec.src_width,
            spec.src_height,
            spec.dst_format,
            spec.dst_width,
            spec.dst_height,
            spec.flags,
        )
        .map_err(|err| format!("scaler create failed: {err}"))?,
    );
    Ok(())
}

#[derive(Clone, Copy)]
pub struct ScalerSpec {
    pub src_format: format::pixel::Pixel,
    pub src_width: u32,
    pub src_height: u32,
    pub dst_format: format::pixel::Pixel,
    pub dst_width: u32,
    pub dst_height: u32,
    pub flags: Flags,
}

pub fn transfer_hw_frame_if_needed(decoded: &frame::Video) -> Result<frame::Video, String> {
    if !is_hardware_frame(decoded)? {
        let mut software_frame = decoded.clone();
        apply_visible_cropping(&mut software_frame)?;
        return Ok(software_frame);
    }
    let mut sw_frame = frame::Video::empty();
    // SAFETY: Both frame pointers are owned AVFrame instances. `sw_frame` is empty output
    // buffer and `decoded` is a valid decoded frame from FFmpeg.
    let ret = unsafe { ffi::av_hwframe_transfer_data(sw_frame.as_mut_ptr(), decoded.as_ptr(), 0) };
    if ret < 0 {
        return Err(format!("hwframe transfer failed: {ret}"));
    }
    // Keep timing/color metadata stable across hw->sw transfer for scaler + renderer decisions.
    let props_ret = unsafe { ffi::av_frame_copy_props(sw_frame.as_mut_ptr(), decoded.as_ptr()) };
    if props_ret < 0 {
        return Err(format!("frame property copy failed: {props_ret}"));
    }
    apply_visible_cropping(&mut sw_frame)?;
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

fn apply_visible_cropping(frame: &mut frame::Video) -> Result<(), String> {
    // Some hardware decoders expose coded-size padding with crop metadata.
    // If not applied, top padding rows can appear as green/magenta flicker.
    let ret = unsafe { ffi::av_frame_apply_cropping(frame.as_mut_ptr(), 0) };
    if ret < 0 {
        return Err(format!("frame cropping failed: {ret}"));
    }
    Ok(())
}

pub fn detect_color_profile(frame: &frame::Video) -> ColorProfile {
    let (color_matrix, y_offset, y_scale, uv_offset, uv_scale) = color_conversion_params(frame);
    ColorProfile {
        color_matrix,
        y_offset,
        y_scale,
        uv_offset,
        uv_scale,
    }
}

pub fn video_frame_to_nv12_planes_from_yuv420p(
    frame: &frame::Video,
    pts_seconds: Option<f64>,
    color_profile: Option<ColorProfile>,
) -> Result<VideoFrame, String> {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    if width == 0 || height == 0 {
        return Err("invalid frame dimension for nv12 extraction".to_string());
    }

    let y_stride = frame.stride(0);
    let y_data = frame.data(0);
    if y_stride < width {
        return Err(format!("invalid y stride: stride={y_stride} width={width}"));
    }
    let y_required = y_stride.saturating_mul(height);
    if y_data.len() < y_required {
        return Err(format!(
            "insufficient y plane bytes: have={} need={}",
            y_data.len(),
            y_required
        ));
    }
    let mut y_plane = Vec::with_capacity(width * height);
    for y in 0..height {
        let start = y * y_stride;
        let end = start + width;
        y_plane.extend_from_slice(&y_data[start..end]);
    }

    let uv_height = height / 2;
    let uv_row_bytes = width / 2;
    let u_stride = frame.stride(1);
    let u_data = frame.data(1);
    let v_stride = frame.stride(2);
    let v_data = frame.data(2);
    if uv_height > 0 {
        if u_stride < uv_row_bytes || v_stride < uv_row_bytes {
            return Err(format!(
                "invalid u/v stride: u_stride={u_stride} v_stride={v_stride} row_bytes={uv_row_bytes}"
            ));
        }
        let u_required = u_stride.saturating_mul(uv_height);
        let v_required = v_stride.saturating_mul(uv_height);
        if u_data.len() < u_required || v_data.len() < v_required {
            return Err(format!(
                "insufficient u/v plane bytes: u_have={} u_need={} v_have={} v_need={}",
                u_data.len(),
                u_required,
                v_data.len(),
                v_required
            ));
        }
    }
    let mut uv_plane = Vec::with_capacity(width * uv_height);
    for y in 0..uv_height {
        let u_row_start = y * u_stride;
        let v_row_start = y * v_stride;
        for x in 0..uv_row_bytes {
            uv_plane.push(u_data[u_row_start + x]);
            uv_plane.push(v_data[v_row_start + x]);
        }
    }

    let profile = color_profile.unwrap_or_else(|| detect_color_profile(frame));
    Ok(VideoFrame {
        pts_seconds: pts_seconds.unwrap_or(0.0),
        width: frame.width(),
        height: frame.height(),
        y_plane,
        uv_plane,
        color_matrix: profile.color_matrix,
        y_offset: profile.y_offset,
        y_scale: profile.y_scale,
        uv_offset: profile.uv_offset,
        uv_scale: profile.uv_scale,
    })
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
