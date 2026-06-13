use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::playback::session::player_settings;
use crate::app::media::state;
use crate::app::media::state::{CacheRecorderSession, MediaState};
use std::fs;
use std::sync::atomic::Ordering;
use tauri::State;

pub(super) fn finalize_active_cache_recording(
    state: &State<'_, MediaState>,
    reason: &str,
) -> MediaResult<()> {
    let mut guard = state
        .cache
        .recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    let Some(mut session) = guard.take() else {
        return Ok(());
    };
    session.active = false;
    session.error_message = Some(reason.to_string());
    if !session.output_path.is_empty() {
        let metadata = fs::metadata(&session.output_path)
            .map_err(|err| MediaError::internal(format!("read cache file failed: {err}")))?;
        if metadata.len() == 0 {
            let _ = fs::remove_file(&session.output_path);
        }
    }
    Ok(())
}

pub(super) fn is_network_source(source: &str) -> bool {
    source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("rtsp://")
        || source.starts_with("rtmp://")
        || source.starts_with("mms://")
}

pub(super) fn set_pending_seek(
    state: &State<'_, MediaState>,
    position_seconds: f64,
) -> MediaResult<()> {
    state
        .runtime
        .stream
        .set_pending_seek_seconds(position_seconds.max(0.0))
}

pub(super) fn activate_playback_and_resume_position(
    state: &State<'_, MediaState>,
) -> MediaResult<(Option<String>, f64)> {
    let latest = state::playback(state)?
        .state()
        .position_seconds
        .max(
            state
                .runtime
                .stream
                .latest_position_seconds()?,
        )
        .max(0.0);
    let current_path = {
        let mut playback = state::playback(state)?;
        playback.play();
        playback.seek(latest);
        playback.state().current_path
    };
    Ok((current_path, latest))
}

pub(super) fn sync_pause_resume_position(state: &State<'_, MediaState>) -> MediaResult<()> {
    let latest = state
        .runtime
        .stream
        .latest_position_seconds()?;
    let (current_path, current_position) = {
        let playback = state::playback(state)?;
        (
            playback.state().current_path.clone(),
            playback.state().position_seconds.max(0.0),
        )
    };
    let resume_position = current_position.max(latest).max(0.0);
    if let Some(path) = current_path.as_deref() {
        let duration_seconds = {
            let playback = state::playback(state)?;
            let duration = playback.state().duration_seconds;
            (duration > 0.0).then_some(duration)
        };
        persist_playback_progress_if_enabled(state, path, resume_position, duration_seconds)?;
    }
    {
        let mut playback = state::playback(state)?;
        playback.seek(resume_position);
        playback.pause();
    }
    if current_path.is_some() {
        state
            .runtime
            .paused_seek_epoch
            .fetch_add(1, Ordering::Relaxed);
    }
    Ok(())
}

pub(super) fn persist_playback_progress_if_enabled(
    state: &State<'_, MediaState>,
    path: &str,
    position_seconds: f64,
    duration_seconds: Option<f64>,
) -> MediaResult<()> {
    if !player_settings::should_persist_playback_progress(state) {
        return Ok(());
    }
    state::library(state)?.mark_playback_progress(path, position_seconds, duration_seconds);
    Ok(())
}

pub(super) fn persist_current_source_before_switch(
    state: &State<'_, MediaState>,
    next_path: &str,
) -> MediaResult<()> {
    if !player_settings::should_persist_playback_progress(state) {
        return Ok(());
    }
    let latest = state.runtime.stream.latest_position_seconds()?;
    let (current_path, current_position, duration_seconds) = {
        let playback = state::playback(state)?;
        let playback_state = playback.state();
        (
            playback_state.current_path.clone(),
            playback_state.position_seconds.max(0.0),
            playback_state.duration_seconds,
        )
    };
    let Some(current_path) = current_path else {
        return Ok(());
    };
    if current_path == next_path {
        return Ok(());
    }
    let position_seconds = current_position.max(latest).max(0.0);
    let duration = (duration_seconds > 0.0).then_some(duration_seconds);
    persist_playback_progress_if_enabled(state, &current_path, position_seconds, duration)
}

pub(super) fn create_cache_recorder_session(
    source: String,
    output_path: String,
    started_at_ms: u64,
) -> CacheRecorderSession {
    CacheRecorderSession {
        source,
        output_path,
        started_at_ms,
        active: true,
        fallback_transcoding: false,
        error_message: None,
    }
}
