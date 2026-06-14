use crate::app::media::playback::decode_context::probe_source_metadata;
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::lyrics::clear_local_lyrics_cache_after_embed;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::PlaybackStatus;
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::runtime::emit_metadata_payloads;
use crate::app::media::playback::session::service::MediaSourceMetadata;
use crate::app::media::playback::session::source_path::{
    normalize_local_image_path, normalize_local_source_path, normalize_playable_source,
};
use crate::app::media::state;
use crate::app::media::state::emit_snapshot_with_request_id;
use crate::app::media::state::MediaState;
use crate::app::media::tags::{
    cover_art_to_base64, read_audio_cover_art, read_image_file_for_cover, supports_tag_writing,
    write_audio_tags, AudioCoverArtData, AudioTagWriteInput, CoverArtChange,
};
use tauri::{AppHandle, State};

use super::session_ops::restart_active_playback;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioCoverArtPayload {
    pub mime_type: String,
    pub data_base64: String,
}

impl From<AudioCoverArtData> for AudioCoverArtPayload {
    fn from(value: AudioCoverArtData) -> Self {
        let data_base64 = cover_art_to_base64(&value);
        Self {
            mime_type: value.mime_type,
            data_base64,
        }
    }
}

pub fn read_audio_cover_art_for_path(
    state: State<'_, MediaState>,
    path: String,
) -> MediaResult<Option<AudioCoverArtPayload>> {
    let normalized_path = normalize_local_source_path(path)?;
    if !supports_tag_writing(&normalized_path) {
        return Err(MediaError::invalid_input(
            "当前音频格式不支持读取封面，请使用 MP3、FLAC、M4A 等常见格式",
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
            "只能读取当前正在播放的本地音频封面",
        ));
    }

    read_audio_cover_art(&normalized_path)
        .map(|value| value.map(Into::into))
        .map_err(MediaError::internal)
}

pub fn read_image_file_for_cover_preview(path: String) -> MediaResult<AudioCoverArtPayload> {
    let normalized_path = normalize_local_image_path(path)?;
    read_image_file_for_cover(&normalized_path)
        .map(Into::into)
        .map_err(MediaError::internal)
}

pub fn write_audio_metadata(
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

    let cover_change = CoverArtChange::parse(cover_art_change.as_deref());
    if cover_change == CoverArtChange::Replace {
        let data = cover_art_data_base64.as_deref().unwrap_or("").trim();
        if data.is_empty() {
            return Err(MediaError::invalid_input(
                "已选择更换封面，请先选择图片后再保存",
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
            cover_art_change: cover_change,
            cover_art_data_base64: cover_art_data_base64.clone(),
            cover_art_mime_type: cover_art_mime_type.clone(),
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
