//! Centralized `MediaState` mutex acquisition with stable error strings.
//!
//! Use this instead of ad-hoc `.lock().map_err(...)` in commands and coordinators so
//! lock ordering and poison messages stay consistent when the player grows.

use std::sync::MutexGuard;

use tauri::State;

use crate::app::media::error::MediaError;
use crate::app::media::library::MediaLibraryService;
use crate::app::media::player::playback::MediaPlaybackService;
use crate::app::media::player::state::MediaState;

pub fn playback<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaPlaybackService>, String> {
    state
        .playback
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())
}

pub fn library<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaLibraryService>, String> {
    state
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())
}

pub fn latest_stream_position_seconds<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, f64>, String> {
    state
        .latest_stream_position_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("latest position state").to_string())
}

pub fn pending_seek_seconds<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, Option<f64>>, String> {
    state
        .pending_seek_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("pending seek state").to_string())
}
