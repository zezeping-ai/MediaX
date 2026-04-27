use crate::app::media::error::MediaResult;
use crate::app::media::playback::runtime::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
};
use crate::app::media::state;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

pub(super) fn restart_active_playback(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: Option<String>,
) -> MediaResult<()> {
    let Some(source) = source else {
        return Ok(());
    };

    let resume_position = resolve_restart_position(state)?;
    super::set_pending_seek(state, resume_position)?;
    state.runtime.stream.advance_generation();
    stop_decode_stream_blocking(state)?;
    start_decode_stream(app, state, source)?;
    Ok(())
}

fn resolve_restart_position(state: &State<'_, MediaState>) -> MediaResult<f64> {
    let latest = read_latest_stream_position(state).unwrap_or(0.0);
    let mut playback = state::playback(state)?;
    let resume_position = playback.state().position_seconds.max(latest).max(0.0);
    playback.seek(resume_position);
    Ok(resume_position)
}
