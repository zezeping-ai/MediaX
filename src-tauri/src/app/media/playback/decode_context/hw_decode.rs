use super::types::HwDecodeStatus;
use crate::app::media::playback::dto::HardwareDecodeMode;
use ffmpeg_next::codec;
use ffmpeg_next::ffi;
use std::ffi::CString;
use std::ptr;

pub(super) fn configure_hw_decode(
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

fn try_bind_hw_device(
    codec_context: &mut codec::context::Context,
    backend: &str,
) -> Result<(), String> {
    let backend_name = CString::new(backend).map_err(|_| "invalid backend name".to_string())?;
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
