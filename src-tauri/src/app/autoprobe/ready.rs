use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::dto::PlaybackStatus;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use std::time::Duration;
use tauri::AppHandle;
use tauri::Manager;

use super::util::now_unix_ms;

pub(super) fn wait_for_playback_ready(
    app: &AppHandle,
    source: &str,
    reason: &str,
    timeout: Duration,
    poll_interval: Duration,
) {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if playback_is_ready(app, source) {
            append_playback_debug_log(
                app,
                now_unix_ms(),
                "autoprobe_ready",
                &format!("{reason} after {}ms", start.elapsed().as_millis()),
            );
            return;
        }
        std::thread::sleep(poll_interval);
    }
    append_playback_debug_log(
        app,
        now_unix_ms(),
        "autoprobe_ready_timeout",
        &format!("{reason} timed out after {}ms", timeout.as_millis()),
    );
}

fn playback_is_ready(app: &AppHandle, source: &str) -> bool {
    let state = app.state::<MediaState>();
    let playback_state = match state::playback(&state) {
        Ok(playback) => playback.state(),
        Err(_) => return false,
    };
    if playback_state.current_path.as_deref() != Some(source) {
        return false;
    }
    if playback_state.status != PlaybackStatus::Playing {
        return false;
    }
    if playback_state.duration_seconds > 0.0 && playback_state.position_seconds > 0.0 {
        return true;
    }
    state
        .runtime
        .stream
        .latest_position_seconds()
        .map(|position| position > 0.0)
        .unwrap_or(false)
}

