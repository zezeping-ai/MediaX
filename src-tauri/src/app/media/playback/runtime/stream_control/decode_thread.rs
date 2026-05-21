use super::error_events::emit_error_events;
use crate::app::media::error::MediaErrorCode;
use crate::app::media::playback::dto::HardwareDecodeMode;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::{DecodeDependencies, DecodeRequest};
use crate::app::media::state::{
    emit_playback_state_snapshot, library, playback, AudioControls, MediaState, TimingControls,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Manager, State};

pub(super) fn spawn_decode_stream(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: String,
    stream_generation: u32,
) -> Result<(Arc<AtomicBool>, thread::JoinHandle<()>), String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_decode = stop_flag.clone();
    let renderer = {
        let handle = app.clone();
        (*handle.state::<RendererState>()).clone()
    };
    let audio_controls: Arc<AudioControls> = state.controls.audio.clone();
    let timing_controls: Arc<TimingControls> = state.controls.timing.clone();
    let app_handle = app.clone();
    let requested_hw_mode = read_requested_hw_mode(&app_handle)?;
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
            mark_decode_stream_failed(&app_handle, stream_generation, &err);
            super::super::emit_debug(&app_handle, "decode_error", err.clone());
            emit_error_events(&app_handle, MediaErrorCode::DecodeFailed.as_str(), err);
        }
    });
    Ok((stop_flag, handle))
}

fn read_requested_hw_mode(app: &AppHandle) -> Result<HardwareDecodeMode, String> {
    let media_state = app.state::<MediaState>();
    let playback = media_state
        .session
        .playback
        .lock()
        .map_err(|_| "playback state poisoned".to_string())?;
    Ok(playback.hw_decode_mode())
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
    let dependencies = DecodeDependencies {
        app,
        renderer,
        stop_flag,
        audio_controls,
        timing_controls,
    };
    let request = DecodeRequest {
        source,
        stream_generation,
        hw_mode_override: requested_hw_mode,
        software_fallback_reason: None,
        force_audio_only: false,
    };
    match super::super::decode_and_emit_stream(dependencies, request) {
        Ok(()) => Ok(()),
        Err(err)
            if requested_hw_mode == HardwareDecodeMode::Auto
                && should_retry_as_software(app, source, stream_generation) =>
        {
            super::super::emit_debug(
                app,
                "hw_decode_fallback",
                format!("runtime fallback to software decode: {err}"),
            );
            let fallback_reason = format!("auto fallback after hardware runtime failure: {err}");
            let fallback_request = request.software_fallback(&fallback_reason);
            super::super::decode_and_emit_stream(dependencies, fallback_request)
        }
        Err(err) => Err(err),
    }
}

fn mark_decode_stream_failed(app: &AppHandle, stream_generation: u32, err: &str) {
    let media_state = app.state::<MediaState>();
    if !media_state
        .runtime
        .stream
        .is_generation_current(stream_generation)
    {
        return;
    }
    let snapshot = (|| -> Result<_, String> {
        let library = library(&media_state)?.state();
        let mut playback = playback(&media_state)?;
        playback.pause();
        playback.update_hw_decode_status(false, None, Some(err.to_string()));
        Ok(playback.snapshot(library))
    })();
    match snapshot {
        Ok(snapshot) => {
            let _ = emit_playback_state_snapshot(app, snapshot, None);
        }
        Err(lock_err) => {
            super::super::emit_debug(app, "decode_error_snapshot_failed", lock_err);
        }
    }
}

fn should_retry_as_software(app: &AppHandle, source: &str, stream_generation: u32) -> bool {
    if !app
        .state::<MediaState>()
        .runtime
        .stream
        .is_generation_current(stream_generation)
    {
        return false;
    }
    let media_state = app.state::<MediaState>();
    let Ok(playback) = media_state.session.playback.lock() else {
        return false;
    };
    let state = playback.state();
    state.current_path.as_deref() == Some(source) && state.hw_decode_active
}
