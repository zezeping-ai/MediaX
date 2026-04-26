use crate::app::media::error::MediaError;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager};

pub(crate) fn current_recording_target(
    app: &AppHandle,
    source: &str,
) -> Result<Option<String>, String> {
    let media_state = app.state::<MediaState>();
    let guard = media_state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder").to_string())?;
    Ok(guard.as_ref().and_then(|session| {
        (session.active && session.source == source).then(|| session.output_path.clone())
    }))
}
