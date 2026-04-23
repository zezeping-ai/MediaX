use crate::app::media::library::MediaLibraryService;
use crate::app::media::playback::MediaPlaybackService;
use crate::app::media::types::MediaSnapshot;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

const MEDIA_STATE_EVENT: &str = "media://state";

#[derive(Default)]
pub struct MediaState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
}

#[tauri::command]
pub fn media_get_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    snapshot_from_state(&state)
}

#[tauri::command]
pub fn media_set_library_roots(
    app: AppHandle,
    state: State<'_, MediaState>,
    roots: Vec<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.set_roots_and_scan(roots);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_rescan_library(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.rescan();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.open(path.clone());
    }
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, 0.0);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_play(app: AppHandle, state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.play();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.pause();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_stop(app: AppHandle, state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.stop();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
) -> Result<MediaSnapshot, String> {
    let path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.seek(position_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.set_rate(playback_rate);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
) -> Result<MediaSnapshot, String> {
    let path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.sync_position(position_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot(&app, &state)
}

fn emit_snapshot(app: &AppHandle, state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let snapshot = snapshot_from_state(state)?;
    app.emit(MEDIA_STATE_EVENT, &snapshot)
        .map_err(|err| format!("emit media state failed: {err}"))?;
    Ok(snapshot)
}

fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .library
        .lock()
        .map_err(|_| "media library state poisoned".to_string())?
        .state();
    let playback = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.state()
    };
    Ok(MediaSnapshot { playback, library })
}
