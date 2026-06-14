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

#[tauri::command]
pub async fn playback_search_lyrics(
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_seconds: Option<f64>,
) -> Result<Vec<crate::app::media::model::LyricsSearchHit>, MediaCommandError> {
    command_result(
        crate::app::media::lyrics::search_lyrics_hits(
            &title,
            artist.as_deref(),
            album.as_deref(),
            duration_seconds.unwrap_or(0.0),
        )
        .await
        .map_err(crate::app::media::error::MediaError::internal),
    )
}

#[tauri::command]
pub fn playback_read_audio_cover_art(
    state: State<'_, MediaState>,
    path: String,
) -> Result<Option<coordinator::AudioCoverArtPayload>, MediaCommandError> {
    command_result(coordinator::read_audio_cover_art_for_path(state, path))
}

#[tauri::command]
pub fn playback_read_image_file_for_cover(
    path: String,
) -> Result<coordinator::AudioCoverArtPayload, MediaCommandError> {
    command_result(coordinator::read_image_file_for_cover_preview(path))
}

#[tauri::command]
pub fn playback_write_audio_metadata(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    lyrics_lrc: Option<String>,
    embed_lyrics: Option<bool>,
    cover_art_change: Option<String>,
    cover_art_data_base64: Option<String>,
    cover_art_mime_type: Option<String>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, MediaCommandError> {
    command_result(coordinator::write_audio_metadata(
        app,
        state,
        path,
        title,
        artist,
        album,
        lyrics_lrc,
        embed_lyrics,
        cover_art_change,
        cover_art_data_base64,
        cover_art_mime_type,
        request_id,
    ))
}
