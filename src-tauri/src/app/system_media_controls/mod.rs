//! OS-level media controls (headphone keys, Control Center, SMTC, MPRIS).

#[cfg(desktop)]
mod inner;

#[cfg(desktop)]
pub use inner::{setup, sync_playback_state};

#[cfg(not(desktop))]
use crate::app::media::playback::dto::PlaybackState;
#[cfg(not(desktop))]
use tauri::AppHandle;

#[cfg(not(desktop))]
pub fn setup(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(not(desktop))]
pub fn sync_playback_state(_app: &AppHandle, _playback: &PlaybackState) {}
