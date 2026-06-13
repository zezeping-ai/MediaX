use super::command_result;
use crate::app::media::error::MediaCommandError;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::session::coordinator;
use crate::app::media::playback::session::player_settings;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn playback_get_snapshot(
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::get_snapshot(state))
}

#[tauri::command]
pub fn playback_open_source(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    resume_last_position: Option<bool>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::open(
        app,
        state,
        path,
        request_id,
        resume_last_position,
    ))
}

#[tauri::command]
pub fn playback_set_resume_last_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    enabled: bool,
) -> Result<(), MediaCommandError> {
    command_result(player_settings::set_resume_last_position(&app, &state, enabled))
}

#[tauri::command]
pub fn playback_select_lyrics_candidate(
    app: AppHandle,
    state: State<'_, MediaState>,
    candidate_id: String,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result({
        crate::app::media::lyrics::select_lyrics_candidate(&app, candidate_id)?;
        coordinator::get_snapshot(state)
    })
}

#[tauri::command]
pub fn playback_set_lyrics_fetch_settings(
    app: AppHandle,
    auto_fetch_online_lyrics: bool,
    providers: player_settings::LyricsProviderSettings,
    lrc_api_base_url: String,
) -> Result<(), MediaCommandError> {
    command_result(player_settings::set_lyrics_fetch_settings(
        &app,
        player_settings::LyricsFetchSettings {
            auto_fetch_online_lyrics,
            providers,
            lrc_api_base_url,
        },
    ))
}

#[tauri::command]
pub fn playback_resume(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::play(app, state, request_id))
}

#[tauri::command]
pub fn playback_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::pause(app, state, request_id))
}

#[tauri::command]
pub fn playback_stop_session(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::stop(app, state, request_id))
}
