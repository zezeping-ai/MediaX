use super::shared::MAIN_WINDOW_LABEL;
use crate::app::media::model::PlaybackStatus;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use tauri::Manager;

pub fn handle_close_requested(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        // Hide instead of quit so tray / external relaunch can restore the app.
        if window.label() == MAIN_WINDOW_LABEL {
            let app = window.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = app.state::<MediaState>();
                let is_playing = state
                    .session
                    .playback
                    .lock()
                    .ok()
                    .map(|playback_guard| playback_guard.state().status == PlaybackStatus::Playing)
                    .unwrap_or(false);
                if is_playing {
                    let _ = coordinator::pause(app.clone(), state, None);
                }
            });
        }
        api.prevent_close();
        let _ = window.hide();
    }
}
