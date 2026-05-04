use crate::app::media::error::MediaResult;
use crate::app::media::playback::runtime::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager, State};

use super::super::helpers::set_pending_seek;

pub(super) fn restart_active_playback(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: Option<String>,
) -> MediaResult<()> {
    let Some(source) = source else {
        return Ok(());
    };

    let resume_position = resolve_restart_position(state)?;
    emit_debug(
        app,
        "restart_begin",
        format!("restart playback source={source} resume={resume_position:.3}s"),
    );
    set_pending_seek(state, resume_position)?;
    state.runtime.stream.advance_generation();
    let restart_epoch = state.runtime.stream.next_restart_epoch();
    // Run stop(join)+start in background so Tauri command won't freeze when FFmpeg demux stalls.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<MediaState>();
        if !state.runtime.stream.is_restart_epoch_current(restart_epoch) {
            emit_debug(&app, "restart_skipped", "restart aborted by newer request");
            return;
        }
        emit_debug(&app, "restart_join_begin", "stop decode stream (blocking join)");
        if let Err(err) = stop_decode_stream_blocking(&state) {
            if err.contains("join timeout") {
                emit_debug(
                    &app,
                    "restart_join_timeout",
                    format!("decode thread join timeout, continue with degraded restart: {err}"),
                );
            } else {
                emit_debug(&app, "restart_error", format!("stop decode stream failed: {err}"));
                return;
            }
        }
        if !state.runtime.stream.is_restart_epoch_current(restart_epoch) {
            emit_debug(&app, "restart_skipped", "restart aborted after join by newer request");
            return;
        }
        emit_debug(&app, "restart_join_end", "decode stream stopped");
        emit_debug(&app, "restart_stream_start", "start decode stream");
        if let Err(err) = start_decode_stream(&app, &state, source) {
            emit_debug(&app, "restart_error", format!("start decode stream failed: {err}"));
        }
    });
    Ok(())
}

fn resolve_restart_position(state: &State<'_, MediaState>) -> MediaResult<f64> {
    let latest = read_latest_stream_position(state).unwrap_or(0.0);
    let mut playback = state::playback(state)?;
    let resume_position = playback.state().position_seconds.max(latest).max(0.0);
    playback.seek(resume_position);
    Ok(resume_position)
}
