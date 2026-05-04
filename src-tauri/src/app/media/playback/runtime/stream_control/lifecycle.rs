use crate::app::media::state::{MediaState, StreamRuntimeState};
use std::thread;
use tauri::{AppHandle, State};

use super::decode_thread::spawn_decode_stream;
use super::handles::take_decode_stream_handles;
use super::join::join_decode_thread_with_timeout;

/// Request decode stream stop and wait for thread exit.
///
/// Use this when the caller is about to start a new decode stream and must avoid
/// multiple demux loops running concurrently (e.g. switching quality / hw decode).
pub fn stop_decode_stream_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let handles = take_decode_stream_handles(state)?;
    if handles.0.is_none() && handles.1.is_none() {
        return Ok(());
    }
    StreamRuntimeState::request_stop(&handles);
    if let Some(handle) = handles.1 {
        join_decode_thread_with_timeout(handle)?;
    }
    Ok(())
}

/// Request decode stream stop without blocking the caller.
///
/// For network streams (e.g. m3u8), ffmpeg demux can block for a while; joining the
/// decode thread inside a Tauri command would freeze the UI. We detach the join to
/// a background thread to keep pause/stop responsive.
pub fn stop_decode_stream_non_blocking(state: &State<'_, MediaState>) -> Result<(), String> {
    let handles = take_decode_stream_handles(state)?;
    if handles.0.is_none() && handles.1.is_none() {
        return Ok(());
    }
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
    let stream_generation = state.runtime.stream.advance_generation();
    super::super::emit_debug(
        app,
        "stream_start",
        format!("start decode stream: {source}"),
    );
    let (stop_flag, handle) = spawn_decode_stream(app, state, source, stream_generation)?;
    state
        .runtime
        .stream
        .install_decode_stream_handle(stop_flag, handle)
        .map_err(|err| err.to_string())?;
    Ok(())
}
