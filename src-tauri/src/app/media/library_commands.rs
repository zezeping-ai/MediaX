use crate::app::media::error::MediaError;
use crate::app::media::player::state::MediaState;
use crate::app::media::snapshot::emit_snapshot;
use crate::app::media::types::MediaSnapshot;
use tauri::{AppHandle, State};

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
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?;
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
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?;
        library.rescan();
    }
    emit_snapshot(&app, &state)
}
