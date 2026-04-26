use crate::app::media::error::MediaErrorCode;
use crate::app::media::model::HardwareDecodeMode;
use crate::app::media::playback::events::{
    MediaErrorPayload, MediaEventEnvelope, MEDIA_PLAYBACK_ERROR_EVENT, MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::{
    AudioControls, DecodeStreamHandles, MediaState, StreamRuntimeState, TimingControls,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
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
}

fn take_decode_stream_handles(
    state: &State<'_, MediaState>,
) -> Result<DecodeStreamHandles, String> {
    state
        .stream
        .take_decode_stream_handles()
        .map_err(|err| err.to_string())
}

/// Request decode stream stop and wait for thread exit.
///
/// Use this when the caller is about to start a new decode stream and must avoid
/// multiple demux loops running concurrently (e.g. switching quality / hw decode).
pub fn stop_decode_stream_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let handles = take_decode_stream_handles(state)?;
    StreamRuntimeState::request_stop(&handles);
    StreamRuntimeState::join(handles);
    Ok(())
}

/// Request decode stream stop without blocking the caller.
///
/// For network streams (e.g. m3u8), ffmpeg demux can block for a while; joining the
/// decode thread inside a Tauri command would freeze the UI. We detach the join to
/// a background thread to keep pause/stop responsive.
pub fn stop_decode_stream_non_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let handles = take_decode_stream_handles(state)?;
    StreamRuntimeState::request_stop(&handles);
    if let Some(handle) = handles.1 {
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
    let stream_generation = state.stream.advance_generation();
    super::emit_debug(
        app,
        "stream_start",
        format!("start decode stream: {source}"),
    );
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_decode = stop_flag.clone();
    let renderer = {
        let handle = app.clone();
        (*handle.state::<RendererState>()).clone()
    };
    let audio_controls: Arc<AudioControls> = state.audio_controls.clone();
    let timing_controls: Arc<TimingControls> = state.timing_controls.clone();
    let app_handle = app.clone();
    let requested_hw_mode = {
        let media_state = app_handle.state::<MediaState>();
        let playback = media_state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.hw_decode_mode()
    };
    let handle = thread::spawn(move || {
        if let Err(err) = run_decode_stream_with_auto_fallback(
            &app_handle,
            &renderer,
            &source,
            &stop_flag_for_decode,
            &audio_controls,
            &timing_controls,
            stream_generation,
            requested_hw_mode,
        ) {
            if let Ok(mut playback) = app_handle.state::<MediaState>().playback.lock() {
                playback.update_hw_decode_status(false, None, Some(err.clone()));
            }
            super::emit_debug(&app_handle, "decode_error", err.clone());
            emit_error_events(&app_handle, MediaErrorCode::DecodeFailed.as_str(), err);
        }
    });
    state
        .stream
        .install_decode_stream_handle(stop_flag, handle)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn run_decode_stream_with_auto_fallback(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
    stream_generation: u32,
    requested_hw_mode: HardwareDecodeMode,
) -> Result<(), String> {
    match super::decode_and_emit_stream(
        app,
        renderer,
        source,
        stop_flag,
        audio_controls,
        timing_controls,
        stream_generation,
        requested_hw_mode,
        None,
    ) {
        Ok(()) => Ok(()),
        Err(err)
            if requested_hw_mode == HardwareDecodeMode::Auto
                && should_retry_as_software(app, source, stream_generation) =>
        {
            super::emit_debug(
                app,
                "hw_decode_fallback",
                format!("runtime fallback to software decode: {err}"),
            );
            super::decode_and_emit_stream(
                app,
                renderer,
                source,
                stop_flag,
                audio_controls,
                timing_controls,
                stream_generation,
                HardwareDecodeMode::Off,
                Some(format!("auto fallback after hardware runtime failure: {err}")),
            )
        }
        Err(err) => Err(err),
    }
}

fn should_retry_as_software(app: &AppHandle, source: &str, stream_generation: u32) -> bool {
    if !app
        .state::<MediaState>()
        .stream
        .is_generation_current(stream_generation)
    {
        return false;
    }
    let media_state = app.state::<MediaState>();
    let Ok(mut playback) = media_state.playback.lock() else {
        return false;
    };
    let state = playback.state();
    state.current_path.as_deref() == Some(source) && state.hw_decode_active
}

pub fn write_latest_stream_position(
    state: &State<'_, MediaState>,
    position_seconds: f64,
) -> Result<(), String> {
    state
        .stream
        .set_latest_position_seconds(position_seconds)
        .map_err(|err| err.to_string())
}

pub fn read_latest_stream_position(state: &State<'_, MediaState>) -> Result<f64, String> {
    state
        .stream
        .latest_position_seconds()
        .map_err(|err| err.to_string())
}
