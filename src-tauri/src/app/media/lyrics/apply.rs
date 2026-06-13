use crate::app::media::error::MediaError;
use crate::app::media::model::{LyricsCandidateSummary, MediaLyricLine};
use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::runtime::emit_metadata_payloads;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct LyricsMetadataContext {
    pub media_kind: PlaybackMediaKind,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub has_cover_art: bool,
}

#[derive(Clone)]
pub struct LyricsPatch {
    pub lyrics: Vec<MediaLyricLine>,
    pub lyrics_source: Option<String>,
    pub lyrics_fetching: bool,
    pub lyrics_candidate_id: Option<String>,
    pub lyrics_candidates: Vec<LyricsCandidateSummary>,
    pub stream_generation: u32,
}

#[derive(Clone)]
pub struct LyricsSelection {
    pub lyrics: Vec<MediaLyricLine>,
    pub lyrics_source: Option<String>,
    pub lyrics_fetching: bool,
    pub lyrics_candidate_id: Option<String>,
    pub lyrics_candidates: Vec<LyricsCandidateSummary>,
}

pub fn patch_lyrics_with_candidates(
    app: &AppHandle,
    context: &LyricsMetadataContext,
    patch: LyricsPatch,
) -> Result<(), String> {
    let media_state = app.state::<MediaState>();
    if !media_state
        .runtime
        .stream
        .is_generation_current(patch.stream_generation)
    {
        return Ok(());
    }

    apply_lyrics_selection(
        app,
        context,
        LyricsSelection {
            lyrics: patch.lyrics,
            lyrics_source: patch.lyrics_source,
            lyrics_candidate_id: patch.lyrics_candidate_id,
            lyrics_candidates: patch.lyrics_candidates,
            lyrics_fetching: patch.lyrics_fetching,
        },
    )
}

pub fn apply_lyrics_selection(
    app: &AppHandle,
    context: &LyricsMetadataContext,
    selection: LyricsSelection,
) -> Result<(), String> {
    let media_state = app.state::<MediaState>();
    {
        let mut playback = media_state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.patch_lyrics(
            selection.lyrics.clone(),
            selection.lyrics_source.clone(),
            selection.lyrics_fetching,
            selection.lyrics_candidate_id.clone(),
            selection.lyrics_candidates.clone(),
        );
    }

    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            media_kind: context.media_kind,
            width: context.width,
            height: context.height,
            fps: context.fps,
            duration_seconds: context.duration_seconds,
            title: context.title.clone(),
            artist: context.artist.clone(),
            album: context.album.clone(),
            has_cover_art: context.has_cover_art,
            lyrics: selection.lyrics,
            lyrics_source: selection.lyrics_source,
            lyrics_fetching: selection.lyrics_fetching,
            lyrics_candidate_id: selection.lyrics_candidate_id,
            lyrics_candidates: selection.lyrics_candidates,
        },
    );
    Ok(())
}

pub fn mark_lyrics_fetching(
    app: &AppHandle,
    context: &LyricsMetadataContext,
    stream_generation: u32,
) -> Result<(), String> {
    patch_lyrics_with_candidates(
        app,
        context,
        LyricsPatch {
            lyrics: Vec::new(),
            lyrics_source: None,
            lyrics_fetching: true,
            lyrics_candidate_id: None,
            lyrics_candidates: Vec::new(),
            stream_generation,
        },
    )
}

pub fn clear_lyrics_fetching(
    app: &AppHandle,
    context: &LyricsMetadataContext,
    lyrics: Vec<MediaLyricLine>,
    lyrics_source: Option<String>,
    lyrics_candidate_id: Option<String>,
    lyrics_candidates: Vec<LyricsCandidateSummary>,
    stream_generation: u32,
) -> Result<(), String> {
    patch_lyrics_with_candidates(
        app,
        context,
        LyricsPatch {
            lyrics,
            lyrics_source,
            lyrics_fetching: false,
            lyrics_candidate_id,
            lyrics_candidates,
            stream_generation,
        },
    )
}
