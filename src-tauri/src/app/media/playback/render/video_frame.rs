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

pub fn preferred_scaled_format_for_renderer(frame: &frame::Video) -> format::pixel::Pixel {
    match frame.format() {
        format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => format::pixel::Pixel::P010LE,
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
    let effective_range = resolve_effective_color_range(frame);
    let storage = sample_storage_for_format(frame.format());
    let matrix = yuv_to_rgb_matrix(frame.color_space());
    let (y_offset, y_scale, uv_offset, uv_scale) =
        range_conversion_params(storage, effective_range);
    ColorProfile {
        color_matrix: matrix,
        y_offset,
        y_scale,
        uv_offset,
        uv_scale,
    }
}

fn resolve_effective_color_range(frame: &frame::Video) -> ffmpeg_next::color::Range {
    use ffmpeg_next::color::Range;
    match frame.color_range() {
        Range::JPEG => Range::JPEG,
        Range::MPEG => sample_luma_range_hint(frame, true).unwrap_or(Range::MPEG),
        Range::Unspecified => sample_luma_range_hint(frame, false).unwrap_or(Range::MPEG),
    }
}

fn sample_luma_range_hint(
    frame: &frame::Video,
    conservative: bool,
) -> Option<ffmpeg_next::color::Range> {
    use ffmpeg_next::color::Range;
    let (min_code, max_code) = sample_luma_code_extents(frame)?;
    let bit_depth = bit_depth_for_format(frame.format());
    let level_shift = bit_depth.saturating_sub(8);
    let limited_black = 16u32 << level_shift;
    let limited_white = 235u32 << level_shift;
    if conservative {
        let loose_black = 8u32 << level_shift;
        let loose_white = 245u32 << level_shift;
        if min_code < loose_black || max_code > loose_white {
            return Some(Range::JPEG);
        }
        return None;
    }
    if min_code < limited_black || max_code > limited_white {
        Some(Range::JPEG)
    } else {
        Some(Range::MPEG)
    }
}

fn sample_luma_code_extents(frame: &frame::Video) -> Option<(u32, u32)> {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    if width == 0 || height == 0 {
        return None;
    }
    let row_step = (height / 8).max(1);
    let col_step = (width / 16).max(1);
    match frame.format() {
        format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => {
            let data = frame.data(0);
            let stride = frame.stride(0);
            let mut min_code = u32::MAX;
            let mut max_code = 0u32;
            let mut sampled = false;
            for row in (0..height).step_by(row_step) {
                for col in (0..width).step_by(col_step) {
                    let byte_idx = row * stride + col * 2;
                    if byte_idx + 1 >= data.len() {
                        continue;
                    }
                    let sample = u16::from_le_bytes([data[byte_idx], data[byte_idx + 1]]);
                    let code = (sample >> 6) as u32;
                    min_code = min_code.min(code);
                    max_code = max_code.max(code);
                    sampled = true;
                }
            }
            sampled.then_some((min_code, max_code))
        }
        _ => {
            let data = frame.data(0);
            let stride = frame.stride(0);
            let mut min_code = u8::MAX;
            let mut max_code = 0u8;
            let mut sampled = false;
            for row in (0..height).step_by(row_step) {
                let row_start = row * stride;
                if row_start >= data.len() {
                    break;
                }
                for col in (0..width).step_by(col_step) {
                    let idx = row_start + col;
                    if idx >= data.len() {
                        break;
                    }
                    let code = data[idx];
                    min_code = min_code.min(code);
                    max_code = max_code.max(code);
                    sampled = true;
                }
            }
            sampled.then_some((min_code as u32, max_code as u32))
        }
    }
}

fn yuv_to_rgb_matrix(color_space: ffmpeg_next::color::Space) -> [[f32; 3]; 3] {
    use ffmpeg_next::color::Space;
    match color_space {
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
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SampleStorage {
    bit_depth: u32,
    storage_bits: u32,
}

fn sample_storage_for_format(format: format::pixel::Pixel) -> SampleStorage {
    match format {
        format::pixel::Pixel::P010LE | format::pixel::Pixel::P010BE => SampleStorage {
            bit_depth: 10,
            // P010 stores 10-bit samples in the MSBs of 16-bit words; the renderer uploads R16Unorm.
            storage_bits: 16,
        },
        _ => {
            let bit_depth = bit_depth_for_format(format);
            SampleStorage {
                bit_depth,
                storage_bits: bit_depth,
            }
        }
    }
}

fn range_conversion_params(
    storage: SampleStorage,
    color_range: ffmpeg_next::color::Range,
) -> (f32, f32, f32, f32) {
    use ffmpeg_next::color::Range;
    let storage_max = ((1u64 << storage.storage_bits) - 1) as f32;
    let storage_shift = storage.storage_bits.saturating_sub(storage.bit_depth);
    let chroma_center_code = 1u32 << (storage.bit_depth - 1);
    let chroma_center = ((chroma_center_code << storage_shift) as f32) / storage_max;
    match color_range {
        Range::JPEG => (0.0, 1.0, chroma_center, 1.0),
        _ => {
            let luma_offset = ((16u32 << storage_shift) as f32) / storage_max;
            let luma_span = ((235u32 - 16u32) << storage_shift) as f32;
            let chroma_span = ((224u32 << (storage.bit_depth - 8)) << storage_shift) as f32;
            (
                luma_offset,
                storage_max / luma_span,
                chroma_center,
                storage_max / chroma_span,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_normalized_luma(
        storage: SampleStorage,
        y_offset: f32,
        y_scale: f32,
        code: u32,
    ) -> f32 {
        let storage_shift = storage.storage_bits.saturating_sub(storage.bit_depth);
        let storage_max = ((1u64 << storage.storage_bits) - 1) as f32;
        let raw = ((code << storage_shift) as f32) / storage_max;
        (raw - y_offset) * y_scale
    }

    #[test]
    fn limited_range_8bit_maps_black_and_white_to_unit_span() {
        let storage = SampleStorage {
            bit_depth: 8,
            storage_bits: 8,
        };
        let (y_offset, y_scale, _, _) = range_conversion_params(storage, ffmpeg_next::color::Range::MPEG);
        let black = decode_normalized_luma(storage, y_offset, y_scale, 16);
        let white = decode_normalized_luma(storage, y_offset, y_scale, 235);
        assert!((black - 0.0).abs() < 1e-4);
        assert!((white - 1.0).abs() < 1e-4);
    }

    #[test]
    fn limited_range_p010_maps_black_and_white_to_unit_span() {
        let storage = sample_storage_for_format(format::pixel::Pixel::P010LE);
        let (y_offset, y_scale, _, _) = range_conversion_params(storage, ffmpeg_next::color::Range::MPEG);
        let black = decode_normalized_luma(storage, y_offset, y_scale, 16);
        let white = decode_normalized_luma(storage, y_offset, y_scale, 235);
        assert!((black - 0.0).abs() < 1e-4);
        assert!((white - 1.0).abs() < 1e-4);
    }

    #[test]
    fn full_range_pixels_infer_jpeg_range_for_unspecified_metadata() {
        assert!(matches!(
            sample_luma_range_hint_for_extents(0, 255, 8, false),
            Some(ffmpeg_next::color::Range::JPEG)
        ));
    }

    #[test]
    fn limited_range_pixels_infer_mpeg_range_for_unspecified_metadata() {
        assert!(matches!(
            sample_luma_range_hint_for_extents(32, 220, 8, false),
            Some(ffmpeg_next::color::Range::MPEG)
        ));
    }

    #[test]
    fn conservative_hint_only_overrides_mpeg_when_pixels_clearly_full_range() {
        assert!(matches!(
            sample_luma_range_hint_for_extents(0, 255, 8, true),
            Some(ffmpeg_next::color::Range::JPEG)
        ));
        assert!(sample_luma_range_hint_for_extents(32, 220, 8, true).is_none());
    }

    fn sample_luma_range_hint_for_extents(
        min_code: u32,
        max_code: u32,
        bit_depth: u32,
        conservative: bool,
    ) -> Option<ffmpeg_next::color::Range> {
        use ffmpeg_next::color::Range;
        let level_shift = bit_depth.saturating_sub(8);
        let limited_black = 16u32 << level_shift;
        let limited_white = 235u32 << level_shift;
        if conservative {
            let loose_black = 8u32 << level_shift;
            let loose_white = 245u32 << level_shift;
            if min_code < loose_black || max_code > loose_white {
                return Some(Range::JPEG);
            }
            return None;
        }
        if min_code < limited_black || max_code > limited_white {
            Some(Range::JPEG)
        } else {
            Some(Range::MPEG)
        }
    }
}
