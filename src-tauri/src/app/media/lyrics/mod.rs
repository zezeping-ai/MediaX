mod apply;
mod cache;
mod candidate;
mod lrc;
mod orchestrator;
mod plain;
mod provider;
mod search;
mod selection;
pub(crate) mod text_encoding;
mod track_signature;

use apply::{
    apply_lyrics_selection, clear_lyrics_fetching, mark_lyrics_fetching, patch_lyrics_with_candidates,
    LyricsMetadataContext, LyricsPatch, LyricsSelection,
};
use cache::{clear_cached_lyrics, load_cached_lyrics, save_cached_lyrics, CachedLyricsEntry};
use candidate::{
    contains_cjk, dedupe_candidates, local_lyrics_candidate, merge_candidates, pick_default_candidate_id,
    LyricsCandidate,
};
use orchestrator::fetch_all_candidates;
use provider::user_agent;
use selection::{clear_selected_candidate_id, load_selected_candidate_id, save_selected_candidate_id};
use track_signature::TrackSignature;

use crate::app::media::model::{MediaLyricLine, LyricsCandidateSummary};
use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::session::player_settings::lyrics_fetch_settings;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager};

pub use lrc::{format_lrc_contents, parse_lrc_contents};
pub use search::search_lyrics_hits;

/// 内嵌歌词写入成功后，清除该曲目的在线歌词缓存与手动切换记录。
pub fn clear_local_lyrics_cache_after_embed(
    app: &AppHandle,
    source_path: &str,
    old_title: Option<&str>,
    old_artist: Option<&str>,
    old_album: Option<&str>,
    duration_seconds: f64,
    new_title: Option<&str>,
    new_artist: Option<&str>,
    new_album: Option<&str>,
) -> Result<(), String> {
    let old_signature = TrackSignature::from_metadata(
        source_path,
        old_title,
        old_artist,
        old_album,
        duration_seconds,
    );
    let new_signature = TrackSignature::from_metadata(
        source_path,
        new_title,
        new_artist,
        new_album,
        duration_seconds,
    );
    clear_cached_lyrics(app, &old_signature)?;
    if new_signature.track_name != old_signature.track_name
        || new_signature.artist_name != old_signature.artist_name
        || new_signature.album_name != old_signature.album_name
    {
        clear_cached_lyrics(app, &new_signature)?;
    }
    clear_selected_candidate_id(app, source_path)?;
    Ok(())
}

const EMBEDDED_LOCAL_SOURCE: &str = "embedded";
const LOCAL_LYRIC_SOURCES: &[&str] = &[EMBEDDED_LOCAL_SOURCE, "sidecar"];

fn local_candidate_id(source: &str) -> String {
    format!("local:{source}")
}

/// 文件自带歌词（内嵌/同目录 LRC）且用户未手动切换过时，锁定为本地来源。
fn is_local_lyrics_locked(app: &AppHandle, context: &OnlineLyricsFetchContext) -> bool {
    let Some(local) = context.local_lyrics.as_ref() else {
        return false;
    };
    if !LOCAL_LYRIC_SOURCES.contains(&local.source.as_str()) {
        return false;
    }
    let local_id = local_candidate_id(&local.source);
    !load_selected_candidate_id(app, &context.source)
        .is_some_and(|saved| saved != local_id)
}

fn local_pinned_candidate(context: &OnlineLyricsFetchContext) -> Option<LyricsCandidate> {
    context.local_lyrics.as_ref().and_then(|local| {
        if !LOCAL_LYRIC_SOURCES.contains(&local.source.as_str()) {
            return None;
        }
        Some(local_lyrics_candidate(
            &local.lines,
            Some(local.source.as_str()),
        ))
    })
}

fn user_chose_lyrics_candidate(app: &AppHandle, source_path: &str) -> bool {
    load_selected_candidate_id(app, source_path).is_some()
}

/// 未手动切换前只暴露当前选中项，避免播放器默认弹出多候选下拉。
fn candidate_summaries_for_playback(
    user_chose: bool,
    selected: &LyricsCandidate,
    all_candidates: &[LyricsCandidate],
) -> Vec<LyricsCandidateSummary> {
    if user_chose {
        all_candidates
            .iter()
            .map(LyricsCandidate::summary)
            .collect()
    } else {
        vec![selected.summary()]
    }
}

fn playback_visible_summaries(
    app: &AppHandle,
    source_path: &str,
    selected: &LyricsCandidate,
    all_candidates: &[LyricsCandidate],
) -> Vec<LyricsCandidateSummary> {
    candidate_summaries_for_playback(
        user_chose_lyrics_candidate(app, source_path),
        selected,
        all_candidates,
    )
}

#[derive(Clone)]
pub struct OnlineLyricsFetchContext {
    pub source: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: f64,
    pub media_kind: PlaybackMediaKind,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_cover_art: bool,
    pub local_lyrics: Option<LocalLyricsSeed>,
}

#[derive(Clone)]
pub struct LocalLyricsSeed {
    pub lines: Vec<MediaLyricLine>,
    pub source: String,
}

impl OnlineLyricsFetchContext {
    pub fn from_video_ctx(source: &str, video_ctx: &VideoDecodeContext) -> Self {
        let local_lyrics = if video_ctx.lyrics.is_empty() {
            None
        } else {
            Some(LocalLyricsSeed {
                lines: video_ctx.lyrics.clone(),
                source: video_ctx
                    .lyrics_source
                    .clone()
                    .unwrap_or_else(|| "local".to_string()),
            })
        };
        Self {
            source: source.to_string(),
            title: video_ctx.title.clone(),
            artist: video_ctx.artist.clone(),
            album: video_ctx.album.clone(),
            duration_seconds: video_ctx.duration_seconds,
            media_kind: video_ctx.media_kind,
            width: video_ctx.output_width,
            height: video_ctx.output_height,
            fps: video_ctx.fps_value,
            has_cover_art: video_ctx.has_cover_art,
            local_lyrics,
        }
    }

    fn metadata_context(&self) -> LyricsMetadataContext {
        LyricsMetadataContext {
            media_kind: self.media_kind,
            width: self.width,
            height: self.height,
            fps: self.fps,
            duration_seconds: self.duration_seconds,
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            has_cover_art: self.has_cover_art,
        }
    }
}

pub fn bootstrap_online_lyrics(
    app: &AppHandle,
    source: &str,
    video_ctx: &VideoDecodeContext,
    stream_generation: u32,
) {
    use crate::app::media::playback::dto::PlaybackMediaKind;

    if video_ctx.media_kind != PlaybackMediaKind::Audio {
        return;
    }
    if !lyrics_fetch_settings().auto_fetch_online_lyrics {
        return;
    }

    let context = OnlineLyricsFetchContext::from_video_ctx(source, video_ctx);
    let metadata_context = context.metadata_context();
    let local_locked = is_local_lyrics_locked(app, &context);
    let initial = build_seed_candidates(app, &context);

    if initial.lines.is_empty() && initial.candidates.is_empty() {
        let _ = mark_lyrics_fetching(app, &metadata_context, stream_generation);
    } else if let Some(selected) = initial
        .candidates
        .iter()
        .find(|candidate| initial.candidate_id.as_deref() == Some(candidate.id.as_str()))
        .or_else(|| initial.candidates.first())
    {
        let summaries = playback_visible_summaries(app, source, selected, &initial.candidates);
        let _ = patch_lyrics_with_candidates(
            app,
            &metadata_context,
            LyricsPatch {
                lyrics: initial.lines.clone(),
                lyrics_source: initial.source.clone(),
                lyrics_fetching: !local_locked,
                lyrics_candidate_id: initial.candidate_id.clone(),
                lyrics_candidates: summaries,
                stream_generation,
            },
        );
        if !local_locked {
            persist_seed_cache(app, &context, &initial);
        }
    }

    if !local_locked {
        spawn_online_lyrics_fetch(app.clone(), context, stream_generation);
    }
}

fn persist_seed_cache(
    app: &AppHandle,
    context: &OnlineLyricsFetchContext,
    initial: &SeedLyricsState,
) {
    if initial.candidates.is_empty() {
        return;
    }
    let signature = build_signature(context);
    let cache_entry = CachedLyricsEntry::from_candidates(
        initial.candidates.clone(),
        initial.candidate_id.clone(),
    );
    let _ = save_cached_lyrics(app, &signature, &cache_entry);
}

struct SeedLyricsState {
    lines: Vec<MediaLyricLine>,
    source: Option<String>,
    candidate_id: Option<String>,
    candidates: Vec<LyricsCandidate>,
}

fn build_seed_candidates(app: &AppHandle, context: &OnlineLyricsFetchContext) -> SeedLyricsState {
    if is_local_lyrics_locked(app, context) {
        if let Some(pinned) = local_pinned_candidate(context) {
            return SeedLyricsState {
                lines: pinned.lines.clone(),
                source: Some(pinned.provider_id.clone()),
                candidate_id: Some(pinned.id.clone()),
                candidates: vec![pinned],
            };
        }
    }

    let mut candidates = Vec::new();
    let mut lines = Vec::new();
    let mut source = None;
    let mut candidate_id = None;

    if let Some(local) = &context.local_lyrics {
        let local_candidate =
            local_lyrics_candidate(&local.lines, Some(local.source.as_str()));
        candidate_id = Some(local_candidate.id.clone());
        source = Some(local_candidate.provider_id.clone());
        lines = local.lines.clone();
        candidates.push(local_candidate);
    }

    let signature = build_signature(context);
    if let Some(saved_id) = load_selected_candidate_id(app, &context.source) {
        if let Some(mut cached) = load_cached_lyrics(app, &signature) {
            let _ = cached.select_candidate(&saved_id);
            candidates.extend(cached.to_candidates());
            if let Some(active) = cached.active_candidate() {
                lines = active.lines.clone();
                source = Some(active.provider_id.clone());
                candidate_id = cached.selected_candidate_id.clone().or(candidate_id);
            }
        }
    }

    let candidates = dedupe_candidates(candidates);
    if let Some(saved_id) = load_selected_candidate_id(app, &context.source) {
        if let Some(selected) = candidates.iter().find(|candidate| candidate.id == saved_id) {
            candidate_id = Some(saved_id);
            lines = selected.lines.clone();
            source = Some(selected.provider_id.clone());
        }
    } else if candidate_id.is_none() {
        if let Some(first) = candidates.first() {
            candidate_id = Some(first.id.clone());
            if lines.is_empty() {
                lines = first.lines.clone();
                source = Some(first.provider_id.clone());
            }
        }
    }

    SeedLyricsState {
        lines,
        source,
        candidate_id,
        candidates,
    }
}

fn build_seed_candidates_for_fetch(app: &AppHandle, context: &OnlineLyricsFetchContext) -> Vec<LyricsCandidate> {
    let signature = build_signature(context);
    let mut candidates = Vec::new();
    if let Some(local) = &context.local_lyrics {
        candidates.push(local_lyrics_candidate(
            &local.lines,
            Some(local.source.as_str()),
        ));
    }
    if !is_local_lyrics_locked(app, context) {
        if let Some(cached) = load_cached_lyrics(app, &signature) {
            candidates.extend(cached.to_candidates());
        }
    }
    dedupe_candidates(candidates)
}

fn current_lyrics_candidate_id(app: &AppHandle) -> Option<String> {
    let media_state = app.state::<MediaState>();
    media_state
        .session
        .playback
        .lock()
        .ok()
        .and_then(|playback| playback.state().lyrics_candidate_id.clone())
}

fn apply_merged_candidates(
    app: &AppHandle,
    context: &OnlineLyricsFetchContext,
    signature: &TrackSignature,
    candidates: Vec<LyricsCandidate>,
    stream_generation: u32,
) {
    let metadata_context = context.metadata_context();
    if candidates.is_empty() {
        let _ = clear_lyrics_fetching(
            app,
            &metadata_context,
            Vec::new(),
            None,
            None,
            Vec::new(),
            stream_generation,
        );
        return;
    }

    if is_local_lyrics_locked(app, context) {
        if let Some(pinned) = local_pinned_candidate(context) {
            let cache_entry =
                CachedLyricsEntry::from_candidates(vec![pinned.clone()], Some(pinned.id.clone()));
            let _ = save_cached_lyrics(app, signature, &cache_entry);
            let _ = clear_lyrics_fetching(
                app,
                &metadata_context,
                pinned.lines.clone(),
                Some(pinned.provider_id.clone()),
                Some(pinned.id.clone()),
                vec![pinned.summary()],
                stream_generation,
            );
            return;
        }
    }

    let prefer_cjk = contains_cjk(&signature.track_name) || contains_cjk(&signature.artist_name);
    let saved_candidate_id = load_selected_candidate_id(app, &context.source)
        .or_else(|| current_lyrics_candidate_id(app));
    let Some(selected) = resolve_selected_candidate(
        &candidates,
        saved_candidate_id.as_deref(),
        prefer_cjk,
        signature.duration_seconds,
    ) else {
        let _ = clear_lyrics_fetching(
            app,
            &metadata_context,
            Vec::new(),
            None,
            None,
            Vec::new(),
            stream_generation,
        );
        return;
    };
    let selected_candidate_id = Some(selected.id.clone());
    let cache_entry =
        CachedLyricsEntry::from_candidates(candidates.clone(), selected_candidate_id.clone());
    let _ = save_cached_lyrics(app, signature, &cache_entry);
    let visible = playback_visible_summaries(app, &context.source, &selected, &candidates);
    let _ = clear_lyrics_fetching(
        app,
        &metadata_context,
        selected.lines,
        Some(selected.provider_id),
        selected_candidate_id,
        visible,
        stream_generation,
    );
}

fn resolve_selected_candidate(
    candidates: &[LyricsCandidate],
    saved_candidate_id: Option<&str>,
    prefer_cjk: bool,
    duration_seconds: f64,
) -> Option<LyricsCandidate> {
    let selected_id = pick_default_candidate_id(candidates, saved_candidate_id, prefer_cjk, duration_seconds)?;
    candidates
        .iter()
        .find(|candidate| candidate.id == selected_id)
        .cloned()
}

pub fn spawn_online_lyrics_fetch(
    app: AppHandle,
    context: OnlineLyricsFetchContext,
    stream_generation: u32,
) {
    tauri::async_runtime::spawn(async move {
        let settings = lyrics_fetch_settings();
        if !settings.auto_fetch_online_lyrics {
            return;
        }

        let signature = build_signature(&context);
        let seed_candidates = build_seed_candidates_for_fetch(&app, &context);
        let search_signature = TrackSignature::for_online_search(
            context.title.as_deref(),
            context.artist.as_deref(),
            context.album.as_deref(),
            context.duration_seconds,
        );
        let client = match reqwest::Client::builder()
            .user_agent(user_agent())
            .build()
        {
            Ok(client) => client,
            Err(_) => {
                apply_merged_candidates(
                    &app,
                    &context,
                    &signature,
                    seed_candidates,
                    stream_generation,
                );
                return;
            }
        };

        let online_candidates = if let Some(search_signature) = search_signature {
            fetch_all_candidates(&client, &search_signature, &settings).await
        } else {
            Vec::new()
        };
        let candidates = merge_candidates(seed_candidates, online_candidates);
        apply_merged_candidates(
            &app,
            &context,
            &signature,
            candidates,
            stream_generation,
        );
    });
}

pub fn select_lyrics_candidate(
    app: &AppHandle,
    candidate_id: String,
) -> Result<(), String> {
    let media_state = app.state::<MediaState>();
    let playback_state = {
        let playback = media_state
            .session
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.state()
    };
    let source_path = playback_state
        .current_path
        .clone()
        .ok_or_else(|| "no active source".to_string())?;
    let title = playback_state.title;
    let artist = playback_state.artist;
    let album = playback_state.album;
    let duration_seconds = playback_state.duration_seconds;

    let signature = TrackSignature::from_metadata(
        &source_path,
        title.as_deref(),
        artist.as_deref(),
        album.as_deref(),
        duration_seconds,
    );
    let mut cached = load_cached_lyrics(app, &signature)
        .ok_or_else(|| "lyrics cache not found for current track".to_string())?;
    if cached.select_candidate(&candidate_id).is_none() {
        return Err("lyrics candidate not found".to_string());
    }

    let lines = cached.lines.clone();
    let source = cached.provider_id.clone();
    let summaries = cached.summaries();
    let selected_candidate_id = cached.selected_candidate_id.clone();

    save_cached_lyrics(app, &signature, &cached)?;
    save_selected_candidate_id(app, &source_path, &candidate_id)?;

    let context = OnlineLyricsFetchContext {
        source: source_path,
        title,
        artist,
        album,
        duration_seconds,
        media_kind: PlaybackMediaKind::Audio,
        width: 0,
        height: 0,
        fps: 0.0,
        has_cover_art: playback_state.has_cover_art,
        local_lyrics: None,
    };
    let metadata_context = context.metadata_context();
    apply_lyrics_selection(
        app,
        &metadata_context,
        LyricsSelection {
            lyrics: lines,
            lyrics_source: Some(source),
            lyrics_fetching: false,
            lyrics_candidate_id: selected_candidate_id,
            lyrics_candidates: summaries,
        },
    )
}

fn build_signature(context: &OnlineLyricsFetchContext) -> TrackSignature {
    TrackSignature::from_metadata(
        &context.source,
        context.title.as_deref(),
        context.artist.as_deref(),
        context.album.as_deref(),
        context.duration_seconds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::media::model::MediaLyricLine;

    fn embedded_context() -> OnlineLyricsFetchContext {
        OnlineLyricsFetchContext {
            source: "/music/song.mp3".to_string(),
            title: Some("Song".to_string()),
            artist: Some("Artist".to_string()),
            album: None,
            duration_seconds: 240.0,
            media_kind: PlaybackMediaKind::Audio,
            width: 0,
            height: 0,
            fps: 0.0,
            has_cover_art: false,
            local_lyrics: Some(LocalLyricsSeed {
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "内嵌歌词".to_string(),
                }],
                source: EMBEDDED_LOCAL_SOURCE.to_string(),
            }),
        }
    }

    #[test]
    fn local_pinned_candidate_uses_local_embedded_id() {
        let pinned = local_pinned_candidate(&embedded_context()).expect("embedded candidate");
        assert_eq!(pinned.id, local_candidate_id(EMBEDDED_LOCAL_SOURCE));
        assert_eq!(pinned.provider_id, EMBEDDED_LOCAL_SOURCE);
    }

    #[test]
    fn candidate_summaries_hide_alternates_until_user_choice() {
        let selected = local_lyrics_candidate(
            &[MediaLyricLine {
                time_seconds: 0.0,
                text: "内嵌".to_string(),
            }],
            Some("embedded"),
        );
        let online = LyricsCandidate {
            id: "netease:1".to_string(),
            provider_id: "netease".to_string(),
            label: "在线".to_string(),
            synced: true,
            preview: "在线".to_string(),
            lines: vec![MediaLyricLine {
                time_seconds: 0.0,
                text: "在线".to_string(),
            }],
            track_name: None,
            artist_name: None,
            duration_seconds: None,
        };
        let visible =
            candidate_summaries_for_playback(false, &selected, &[selected.clone(), online]);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, selected.id);
    }
}
