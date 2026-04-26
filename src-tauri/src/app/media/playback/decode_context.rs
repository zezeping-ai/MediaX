use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::media::Type;
use std::ffi::CString;
use std::ptr;

use crate::app::media::model::{HardwareDecodeMode, PlaybackQualityMode};

pub(crate) struct VideoDecodeContext {
    pub(crate) input_ctx: format::context::Input,
    pub(crate) video_stream_index: usize,
    pub(crate) video_time_base: ffmpeg::Rational,
    pub(crate) decoder: ffmpeg::decoder::Video,
    pub(crate) fps_value: f64,
    pub(crate) duration_seconds: f64,
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
    pub(crate) hw_decode_active: bool,
    pub(crate) hw_decode_backend: Option<String>,
    pub(crate) hw_decode_error: Option<String>,
}

pub(crate) fn open_video_decode_context(
    source: &str,
    hw_mode: HardwareDecodeMode,
    quality_mode: PlaybackQualityMode,
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
    let hw_status = configure_hw_decode(&mut codec_context, source, hw_mode)?;
    let decoder = codec_context
        .decoder()
        .video()
        .map_err(|err| format!("video decoder create failed: {err}"))?;
    let (output_width, output_height) =
        compute_output_size(decoder.width(), decoder.height(), quality_mode);
    Ok(VideoDecodeContext {
        input_ctx,
        video_stream_index,
        video_time_base: stream_time_base,
        decoder,
        fps_value,
        duration_seconds,
        output_width,
        output_height,
        hw_decode_active: hw_status.active,
        hw_decode_backend: hw_status.backend,
        hw_decode_error: hw_status.error,
    })
}

struct HwDecodeStatus {
    active: bool,
    backend: Option<String>,
    error: Option<String>,
}

fn preferred_hw_backends() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["videotoolbox"]
    }
    #[cfg(target_os = "windows")]
    {
        &["d3d11va", "dxva2"]
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        &["vaapi", "cuda"]
    }
}

fn configure_hw_decode(
    codec_context: &mut codec::context::Context,
    source: &str,
    hw_mode: HardwareDecodeMode,
) -> Result<HwDecodeStatus, String> {
    if hw_mode == HardwareDecodeMode::Off {
        return Ok(HwDecodeStatus {
            active: false,
            backend: None,
            error: None,
        });
    }
    if should_prefer_software_decode(source, hw_mode) {
        return Ok(HwDecodeStatus {
            active: false,
            backend: None,
            error: Some("prefer software decode for network HLS source".to_string()),
        });
    }
    let mut last_error: Option<String> = None;
    for backend in preferred_hw_backends() {
        match try_bind_hw_device(codec_context, backend) {
            Ok(()) => {
                return Ok(HwDecodeStatus {
                    active: true,
                    backend: Some((*backend).to_string()),
                    error: None,
                });
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }
    if hw_mode == HardwareDecodeMode::On {
        return Err("hardware decode forced but no supported backend available".to_string());
    }
    Ok(HwDecodeStatus {
        active: false,
        backend: None,
        error: last_error,
    })
}

fn should_prefer_software_decode(source: &str, hw_mode: HardwareDecodeMode) -> bool {
    if hw_mode != HardwareDecodeMode::Auto {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        let normalized = source.trim().to_ascii_lowercase();
        let is_network = normalized.starts_with("http://") || normalized.starts_with("https://");
        let is_hls = normalized.contains(".m3u8");
        return is_network && is_hls;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = source;
        false
    }
}

fn try_bind_hw_device(
    codec_context: &mut codec::context::Context,
    backend: &str,
) -> Result<(), String> {
    let backend_name = CString::new(backend).map_err(|_| "invalid backend name".to_string())?;
    // SAFETY: FFmpeg device creation API expects a valid backend name C-string and mutable
    // out-pointer. On success, we immediately ref-count the buffer for codec context and unref
    // the temporary owner to avoid leaks.
    unsafe {
        let device_type = ffi::av_hwdevice_find_type_by_name(backend_name.as_ptr());
        if device_type == ffi::AVHWDeviceType::AV_HWDEVICE_TYPE_NONE {
            return Err(format!("backend not supported by ffmpeg build: {backend}"));
        }
        let mut device_ref: *mut ffi::AVBufferRef = ptr::null_mut();
        let ret = ffi::av_hwdevice_ctx_create(
            &mut device_ref,
            device_type,
            ptr::null(),
            ptr::null_mut(),
            0,
        );
        if ret < 0 || device_ref.is_null() {
            return Err(format!("create hw device failed for backend: {backend}"));
        }
        let codec_ptr = codec_context.as_mut_ptr();
        (*codec_ptr).hw_device_ctx = ffi::av_buffer_ref(device_ref);
        ffi::av_buffer_unref(&mut device_ref);
        if (*codec_ptr).hw_device_ctx.is_null() {
            return Err(format!("bind hw device failed for backend: {backend}"));
        }
    }
    Ok(())
}

fn compute_output_size(width: u32, height: u32, quality_mode: PlaybackQualityMode) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }
    let Some(max_height) = quality_mode_max_height(quality_mode) else {
        let mut out_width = width.max(2);
        let mut out_height = height.max(2);
        out_width &= !1;
        out_height &= !1;
        return (out_width.max(2), out_height.max(2));
    };
    let height_scale = (max_height as f64) / (height as f64);
    let scale = height_scale.min(1.0);
    let mut out_width = ((width as f64) * scale).round().max(2.0) as u32;
    let mut out_height = ((height as f64) * scale).round().max(2.0) as u32;
    out_width &= !1;
    out_height &= !1;
    (out_width.max(2), out_height.max(2))
}

fn quality_mode_max_height(mode: PlaybackQualityMode) -> Option<u32> {
    match mode {
        PlaybackQualityMode::Source | PlaybackQualityMode::Auto => None,
        PlaybackQualityMode::P1080 => Some(1080),
        PlaybackQualityMode::P720 => Some(720),
        PlaybackQualityMode::P480 => Some(480),
        PlaybackQualityMode::P320 => Some(320),
    }
}
