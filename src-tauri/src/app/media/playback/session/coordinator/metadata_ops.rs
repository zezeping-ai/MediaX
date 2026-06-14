use crate::app::media::playback::decode_context::probe_source_metadata;
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::lyrics::clear_local_lyrics_cache_after_embed;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::PlaybackStatus;
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::runtime::emit_metadata_payloads;
use crate::app::media::playback::session::service::MediaSourceMetadata;
use crate::app::media::playback::session::source_path::{
    normalize_local_source_path, normalize_playable_source,
};
use crate::app::media::state;
use crate::app::media::state::emit_snapshot_with_request_id;
use crate::app::media::state::MediaState;
use crate::app::media::tags::{supports_tag_writing, write_audio_tags, AudioTagWriteInput};
use tauri::{AppHandle, State};

use super::session_ops::restart_active_playback;

pub fn write_audio_metadata(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    lyrics_lrc: Option<String>,
    embed_lyrics: Option<bool>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let normalized_path = normalize_local_source_path(path)?;
    if !supports_tag_writing(&normalized_path) {
        return Err(MediaError::invalid_input(
            "当前音频格式不支持写入标签，请使用 MP3、FLAC、M4A 等常见格式",
        ));
    }

    let active_path = {
        let playback = state::playback(&state)?;
        playback
            .state()
            .current_path
            .clone()
            .ok_or_else(|| MediaError::invalid_input("当前没有打开的本地音频"))?
    };
    let normalized_active = normalize_playable_source(active_path)?;
    if normalized_active != normalized_path {
        return Err(MediaError::invalid_input(
            "只能编辑当前正在播放的本地音频文件",
        ));
    }

    let embed_lyrics = embed_lyrics.unwrap_or(false);
    let lyrics_lrc = lyrics_lrc.map(|value| value.trim().to_string());
    if embed_lyrics {
        let lyrics = lyrics_lrc.as_deref().unwrap_or("").trim();
        if lyrics.is_empty() {
            return Err(MediaError::invalid_input(
                "已勾选嵌入文件，请先选择或填写歌词后再保存",
            ));
        }
    }

    let (old_title, old_artist, old_album, duration_seconds) = {
        let playback = state::playback(&state)?;
        let playback_state = playback.state();
        (
            playback_state.title.clone(),
            playback_state.artist.clone(),
            playback_state.album.clone(),
            playback_state.duration_seconds,
        )
    };

    write_audio_tags(
        &normalized_path,
        &AudioTagWriteInput {
            title: title.clone(),
            artist: artist.clone(),
            album: album.clone(),
            lyrics_lrc: lyrics_lrc.clone(),
            embed_lyrics,
        },
    )
    .map_err(MediaError::internal)?;

    if embed_lyrics {
        clear_local_lyrics_cache_after_embed(
            &app,
            &normalized_path,
            old_title.as_deref(),
            old_artist.as_deref(),
            old_album.as_deref(),
            duration_seconds,
            title.as_deref(),
            artist.as_deref(),
            album.as_deref(),
        )
        .map_err(MediaError::internal)?;
    }

    refresh_current_source_metadata(&app, &state)?;

    let playing_path = {
        let playback = state::playback(&state)?;
        if playback.state().status == PlaybackStatus::Playing {
            playback.state().current_path.clone()
        } else {
            None
        }
    };
    restart_active_playback(&app, &state, playing_path)?;

    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

fn refresh_current_source_metadata(
    app: &AppHandle,
    state: &State<'_, MediaState>,
) -> MediaResult<()> {
    let (source, duration_seconds, width, height, fps) = {
        let playback = state::playback(state)?;
        let playback_state = playback.state();
        let Some(source) = playback_state.current_path.clone() else {
            return Ok(());
        };
        (
            source,
            playback_state.duration_seconds,
            0u32,
            0u32,
            0.0f64,
        )
    };

    let probed = probe_source_metadata(&source).map_err(MediaError::internal)?;
    let metadata = MediaSourceMetadata {
        title: probed.title.clone(),
        artist: probed.artist.clone(),
        album: probed.album.clone(),
        has_cover_art: probed.has_cover_art,
        lyrics: probed.lyrics.clone(),
        lyrics_source: probed.lyrics_source.clone(),
        lyrics_fetching: false,
        lyrics_candidate_id: None,
        lyrics_candidates: Vec::new(),
    };

    {
        let mut playback = state::playback(state)?;
        playback.set_media_metadata(metadata.clone());
    }

    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            media_kind: probed.media_kind,
            width,
            height,
            fps,
            duration_seconds,
            title: metadata.title,
            artist: metadata.artist,
            album: metadata.album,
            has_cover_art: metadata.has_cover_art,
            lyrics: metadata.lyrics,
            lyrics_source: metadata.lyrics_source,
            lyrics_fetching: false,
            lyrics_candidate_id: None,
            lyrics_candidates: Vec::new(),
        },
    );
    Ok(())
}
