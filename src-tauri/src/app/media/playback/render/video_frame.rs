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

pub fn preferred_scaling_flags_for_renderer(
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> Flags {
    let src_pixels = u64::from(src_width).saturating_mul(u64::from(src_height));
    let dst_pixels = u64::from(dst_width).saturating_mul(u64::from(dst_height));
    // Large downscales are the hot path for smooth playback on 4K-class sources. Favor
    // a cheaper scaler there so we preserve decode/render cadence under load.
    if src_pixels > 0 && dst_pixels.saturating_mul(2) <= src_pixels {
        Flags::FAST_BILINEAR
    } else {
        Flags::BILINEAR
    }
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

pub fn can_bypass_scaler_for_renderer(
    frame: &frame::Video,
    output_width: u32,
    output_height: u32,
) -> bool {
    frame.width() == output_width
        && frame.height() == output_height
        && matches!(
        frame.format(),
        format::pixel::Pixel::NV12
            | format::pixel::Pixel::P010LE
            | format::pixel::Pixel::P010BE
            | format::pixel::Pixel::YUV420P
    )
}

pub fn adaptive_upload_size_for_renderer(
    src_width: u32,
    src_height: u32,
    surface_width: u32,
    surface_height: u32,
    source_fps: f64,
) -> (u32, u32) {
    if src_width == 0 || src_height == 0 {
        return (src_width, src_height);
    }
    if surface_width == 0 || surface_height == 0 {
        return even_dimensions(src_width, src_height);
    }
    let fit_scale = ((surface_width as f64) / (src_width as f64))
        .min((surface_height as f64) / (src_height as f64))
        .min(1.0);
    let fitted_width = ((src_width as f64) * fit_scale).round().max(2.0) as u32;
    let fitted_height = ((src_height as f64) * fit_scale).round().max(2.0) as u32;
    let source_pixels = (src_width as f64) * (src_height as f64);
    let fitted_pixels = (fitted_width as f64) * (fitted_height as f64);
    let scale_pressure = if fitted_pixels > 0.0 {
        source_pixels / fitted_pixels
    } else {
        1.0
    };
    let fps = if source_fps.is_finite() && source_fps > 1.0 {
        source_fps
    } else {
        24.0
    };
    // Favor smoothness for heavy/high-fps sources by allowing a modest undershoot below the
    // fitted viewport size. Lighter sources retain a bit of supersampling headroom.
    let mut viewport_quality_factor: f64 = if fps >= 50.0 {
        0.88
    } else if fps >= 30.0 {
        0.94
    } else {
        1.0
    };
    if scale_pressure >= 6.0 {
        viewport_quality_factor -= 0.14;
    } else if scale_pressure >= 4.0 {
        viewport_quality_factor -= 0.10;
    } else if scale_pressure >= 2.5 {
        viewport_quality_factor -= 0.04;
    }
    let viewport_quality_factor = viewport_quality_factor.clamp(0.78, 1.08);
    let target_width = ((fitted_width as f64) * viewport_quality_factor)
        .round()
        .clamp(2.0, src_width as f64) as u32;
    let target_height = ((fitted_height as f64) * viewport_quality_factor)
        .round()
        .clamp(2.0, src_height as f64) as u32;
    even_dimensions(target_width, target_height)
}

pub fn preferred_scaled_format_for_renderer(frame: &frame::Video) -> format::pixel::Pixel {
    match frame.format() {
        format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => {
            format::pixel::Pixel::P010LE
        }
        _ => format::pixel::Pixel::NV12,
    }
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

fn color_conversion_params(frame: &frame::Video) -> ([[f32; 3]; 3], f32, f32, f32, f32) {
    use ffmpeg_next::color::{Range, Space};
    let bit_depth = bit_depth_for_format(frame.format()) as f32;
    let sample_max = (1u32 << bit_depth_for_format(frame.format())) as f32 - 1.0;
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
        Range::JPEG => (
            matrix,
            0.0,
            1.0,
            (1u32 << (bit_depth as u32 - 1)) as f32 / sample_max,
            1.0,
        ),
        _ => {
            let luma_offset = (16u32 << (bit_depth as u32 - 8)) as f32 / sample_max;
            let luma_range = (219u32 << (bit_depth as u32 - 8)) as f32;
            let chroma_center = (1u32 << (bit_depth as u32 - 1)) as f32 / sample_max;
            let chroma_range = (224u32 << (bit_depth as u32 - 8)) as f32;
            (
                matrix,
                luma_offset,
                sample_max / luma_range,
                chroma_center,
                sample_max / chroma_range,
            )
        }
    }
}

fn bit_depth_for_format(format: format::pixel::Pixel) -> u32 {
    match format {
        format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => 10,
        _ => 8,
    }
}

fn even_dimensions(width: u32, height: u32) -> (u32, u32) {
    let mut even_width = width.max(2);
    let mut even_height = height.max(2);
    even_width &= !1;
    even_height &= !1;
    (even_width.max(2), even_height.max(2))
}
