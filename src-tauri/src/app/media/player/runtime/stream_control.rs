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

pub fn stop_decode_stream(state: &State<'_, MediaState>) -> Result<(), String> {
    let mut guard = state
        .stream_stop_flag
        .lock()
        .map_err(|_| "stream state poisoned".to_string())?;
    if let Some(flag) = guard.take() {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

pub fn start_decode_stream(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: String,
) -> Result<(), String> {
    stop_decode_stream(state)?;
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
            .map_err(|_| "stream state poisoned".to_string())?;
        *guard = Some(stop_flag.clone());
    }
    let renderer = {
        let handle = app.clone();
        (*handle.state::<RendererState>()).clone()
    };
    let audio_controls: Arc<AudioControls> = state.audio_controls.clone();
    let timing_controls: Arc<TimingControls> = state.timing_controls.clone();
    let app_handle = app.clone();
    thread::spawn(move || {
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
            let emitted_at_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let error_payload = MediaErrorPayload {
                code: "DECODE_FAILED",
                message: err,
            };
            let _ = app_handle.emit(
                MEDIA_PLAYBACK_ERROR_EVENT,
                MediaEventEnvelope {
                    protocol_version: MEDIA_PROTOCOL_VERSION,
                    event_type: "playback_error",
                    request_id: None,
                    emitted_at_ms,
                    payload: error_payload.clone(),
                },
            );
            let _ = app_handle.emit(
                MEDIA_ERROR_EVENT,
                error_payload,
            );
        }
    });
    Ok(())
}

pub fn write_latest_stream_position(
    state: &State<'_, MediaState>,
    position_seconds: f64,
) -> Result<(), String> {
    let mut guard = state
        .latest_stream_position_seconds
        .lock()
        .map_err(|_| "latest position state poisoned".to_string())?;
    *guard = position_seconds.max(0.0);
    Ok(())
}

pub fn read_latest_stream_position(state: &State<'_, MediaState>) -> Result<f64, String> {
    let guard = state
        .latest_stream_position_seconds
        .lock()
        .map_err(|_| "latest position state poisoned".to_string())?;
    Ok((*guard).max(0.0))
}
