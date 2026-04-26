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
    pub(crate) hw_decode_decision: String,
}

pub(crate) fn open_video_decode_context(
    source: &str,
    hw_mode: HardwareDecodeMode,
    quality_mode: PlaybackQualityMode,
    software_fallback_reason: Option<&str>,
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
    let hw_status = configure_hw_decode(&mut codec_context, hw_mode, software_fallback_reason)?;
    let decoder = match codec_context.decoder().video() {
        Ok(decoder) => decoder,
        Err(err) if hw_mode == HardwareDecodeMode::Auto && hw_status.active => {
            let fallback_reason =
                format!("auto fallback to software after decoder open failed: {err}");
            let mut software_context =
                codec::context::Context::from_parameters(input_stream.parameters())
                    .map_err(|ctx_err| format!("decoder context failed: {ctx_err}"))?;
            let software_status = configure_hw_decode(
                &mut software_context,
                HardwareDecodeMode::Off,
                Some(&fallback_reason),
            )?;
            let decoder = software_context
                .decoder()
                .video()
                .map_err(|decode_err| format!("video decoder create failed after fallback: {decode_err}"))?;
            return finalize_video_decode_context(
                input_ctx,
                video_stream_index,
                stream_time_base,
                fps_value,
                duration_seconds,
                quality_mode,
                decoder,
                software_status,
            );
        }
        Err(err) => {
            return Err(format!("video decoder create failed: {err}"));
        }
    };
    finalize_video_decode_context(
        input_ctx,
        video_stream_index,
        stream_time_base,
        fps_value,
        duration_seconds,
        quality_mode,
        decoder,
        hw_status,
    )
}

fn finalize_video_decode_context(
    input_ctx: format::context::Input,
    video_stream_index: usize,
    video_time_base: ffmpeg::Rational,
    fps_value: f64,
    duration_seconds: f64,
    quality_mode: PlaybackQualityMode,
    decoder: ffmpeg::decoder::Video,
    hw_status: HwDecodeStatus,
) -> Result<VideoDecodeContext, String> {
    let (output_width, output_height) =
        compute_output_size(decoder.width(), decoder.height(), quality_mode);
    Ok(VideoDecodeContext {
        input_ctx,
        video_stream_index,
        video_time_base,
        decoder,
        fps_value,
        duration_seconds,
        output_width,
        output_height,
        hw_decode_active: hw_status.active,
        hw_decode_backend: hw_status.backend,
        hw_decode_error: hw_status.error,
        hw_decode_decision: hw_status.decision,
    })
}

struct HwDecodeStatus {
    active: bool,
    backend: Option<String>,
    error: Option<String>,
    decision: String,
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
    hw_mode: HardwareDecodeMode,
    software_fallback_reason: Option<&str>,
) -> Result<HwDecodeStatus, String> {
    if hw_mode == HardwareDecodeMode::Off {
        return Ok(HwDecodeStatus {
            active: false,
            backend: None,
            error: software_fallback_reason.map(ToOwned::to_owned),
            decision: if let Some(reason) = software_fallback_reason {
                format!("software decode selected: {reason}")
            } else {
                "software decode selected by preference".to_string()
            },
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
                    decision: format!("hardware decode selected via backend={backend}"),
                });
            }
            Err(err) => {
                last_error = Some(err);
            }
        }
    }
    if hw_mode == HardwareDecodeMode::On {
        let reason = last_error.unwrap_or_else(|| "no supported backend available".to_string());
        return Err(format!("hardware decode forced but unavailable: {reason}"));
    }
    Ok(HwDecodeStatus {
        active: false,
        backend: None,
        error: last_error.clone(),
        decision: match last_error {
            Some(err) => format!("auto fallback to software before playback start: {err}"),
            None => "auto fallback to software before playback start".to_string(),
        },
    })
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
