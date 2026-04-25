use crate::app::media::error::{MediaError, MediaErrorCode};
use crate::app::media::player::events::{
    MediaErrorPayload, MediaEventEnvelope, MEDIA_ERROR_EVENT, MEDIA_PLAYBACK_ERROR_EVENT,
    MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::player::renderer::RendererState;
use crate::app::media::player::state::{AudioControls, MediaState, TimingControls};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};

fn unix_epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn emit_error_events(app: &AppHandle, code: &'static str, message: String) {
    let emitted_at_ms = unix_epoch_ms_now();
    let error_payload = MediaErrorPayload { code, message };
    let _ = app.emit(
        MEDIA_PLAYBACK_ERROR_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_error",
            request_id: None,
            emitted_at_ms,
            payload: error_payload.clone(),
        },
    );
    let _ = app.emit(MEDIA_ERROR_EVENT, error_payload);
}

fn take_decode_stream_handles(
    state: &State<'_, MediaState>,
) -> Result<(Option<Arc<AtomicBool>>, Option<thread::JoinHandle<()>>), String> {
    let stop_flag = {
        let mut guard = state
            .stream_stop_flag
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream state").to_string())?;
        guard.take()
    };
    let join_handle = {
        let mut guard = state
            .stream_thread
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream thread").to_string())?;
        guard.take()
    };
    Ok((stop_flag, join_handle))
}

/// Request decode stream stop and wait for thread exit.
///
/// Use this when the caller is about to start a new decode stream and must avoid
/// multiple demux loops running concurrently (e.g. switching quality / hw decode).
pub fn stop_decode_stream_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let (stop_flag, join_handle) = take_decode_stream_handles(state)?;
    if let Some(flag) = stop_flag {
        flag.store(true, Ordering::Relaxed);
    }
    if let Some(handle) = join_handle {
        let _ = handle.join();
    }
    Ok(())
}

/// Request decode stream stop without blocking the caller.
///
/// For network streams (e.g. m3u8), ffmpeg demux can block for a while; joining the
/// decode thread inside a Tauri command would freeze the UI. We detach the join to
/// a background thread to keep pause/stop responsive.
pub fn stop_decode_stream_non_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let (stop_flag, join_handle) = take_decode_stream_handles(state)?;
    if let Some(flag) = stop_flag {
        flag.store(true, Ordering::Relaxed);
    }
    if let Some(handle) = join_handle {
        thread::spawn(move || {
            let _ = handle.join();
        });
    }
    Ok(())
}

pub fn start_decode_stream(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: String,
) -> Result<(), String> {
    stop_decode_stream_blocking(state)?;
    super::emit_debug(
        app,
        "stream_start",
        format!("start decode stream: {source}"),
    );
    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = state
            .stream_stop_flag
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream state").to_string())?;
        *guard = Some(stop_flag.clone());
    }
    let renderer = {
        let handle = app.clone();
        (*handle.state::<RendererState>()).clone()
    };
    let audio_controls: Arc<AudioControls> = state.audio_controls.clone();
    let timing_controls: Arc<TimingControls> = state.timing_controls.clone();
    let app_handle = app.clone();
    let handle = thread::spawn(move || {
        if let Err(err) = super::decode_and_emit_stream(
            &app_handle,
            &renderer,
            &source,
            &stop_flag,
            &audio_controls,
            &timing_controls,
        ) {
            if let Ok(mut playback) = app_handle.state::<MediaState>().playback.lock() {
                playback.update_hw_decode_status(false, None, Some(err.clone()));
            }
            super::emit_debug(&app_handle, "decode_error", err.clone());
            emit_error_events(&app_handle, MediaErrorCode::DecodeFailed.as_str(), err);
        }
    });
    {
        let mut guard = state
            .stream_thread
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream thread").to_string())?;
        *guard = Some(handle);
    }
    Ok(())
}

pub fn write_latest_stream_position(
    state: &State<'_, MediaState>,
    position_seconds: f64,
) -> Result<(), String> {
    let mut guard = state
        .latest_stream_position_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("latest position state").to_string())?;
    *guard = position_seconds.max(0.0);
    Ok(())
}

pub fn read_latest_stream_position(state: &State<'_, MediaState>) -> Result<f64, String> {
    let guard = state
        .latest_stream_position_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("latest position state").to_string())?;
    Ok((*guard).max(0.0))
}
