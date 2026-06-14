use crate::app::media::model::{LyricsCandidateSummary, MediaLyricLine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use super::candidate::{build_preview, LyricsCandidate};
use super::track_signature::TrackSignature;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLyricsCandidate {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub synced: bool,
    pub preview: String,
    pub lines: Vec<MediaLyricLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLyricsEntry {
    #[serde(default)]
    pub selected_candidate_id: Option<String>,
    #[serde(default)]
    pub candidates: Vec<CachedLyricsCandidate>,
    #[serde(default)]
    pub provider_id: String,
    #[serde(default)]
    pub synced: bool,
    #[serde(default)]
    pub lines: Vec<MediaLyricLine>,
}

impl CachedLyricsEntry {
    pub fn from_candidates(
        candidates: Vec<LyricsCandidate>,
        selected_candidate_id: Option<String>,
    ) -> Self {
        let cached_candidates = candidates
            .iter()
            .map(|candidate| {
                let summary = candidate.summary();
                CachedLyricsCandidate {
                    id: summary.id,
                    provider_id: summary.provider_id,
                    label: summary.label,
                    synced: summary.synced,
                    preview: summary.preview,
                    lines: candidate.lines.clone(),
                }
            })
            .collect::<Vec<_>>();
        let selected = cached_candidates
            .iter()
            .find(|candidate| Some(candidate.id.as_str()) == selected_candidate_id.as_deref());
        let fallback = cached_candidates.first();
        let active = selected.or(fallback);
        let provider_id = active
            .map(|value| value.provider_id.clone())
            .unwrap_or_default();
        let synced = active.map(|value| value.synced).unwrap_or(false);
        let lines = active
            .map(|value| value.lines.clone())
            .unwrap_or_default();
        Self {
            selected_candidate_id: selected_candidate_id.or_else(|| active.map(|value| value.id.clone())),
            candidates: cached_candidates,
            provider_id,
            synced,
            lines,
        }
    }

    pub fn normalize_legacy(&mut self) {
        if !self.candidates.is_empty() {
            return;
        }
        if self.lines.is_empty() {
            return;
        }
        let id = if self.provider_id.is_empty() {
            "legacy:0".to_string()
        } else {
            format!("{}:legacy", self.provider_id)
        };
        self.candidates.push(CachedLyricsCandidate {
            id: id.clone(),
            provider_id: self.provider_id.clone(),
            label: self.provider_id.clone(),
            synced: self.synced,
            preview: build_preview(&self.lines),
            lines: self.lines.clone(),
        });
        if self.selected_candidate_id.is_none() {
            self.selected_candidate_id = Some(id);
        }
    }

    pub fn summaries(&self) -> Vec<LyricsCandidateSummary> {
        self.candidates
            .iter()
            .map(|candidate| LyricsCandidateSummary {
                id: candidate.id.clone(),
                provider_id: candidate.provider_id.clone(),
                label: candidate.label.clone(),
                synced: candidate.synced,
                preview: candidate.preview.clone(),
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            })
            .collect()
    }

    pub fn active_candidate(&self) -> Option<&CachedLyricsCandidate> {
        if let Some(selected_id) = self.selected_candidate_id.as_deref() {
            if let Some(candidate) = self
                .candidates
                .iter()
                .find(|candidate| candidate.id == selected_id)
            {
                return Some(candidate);
            }
        }
        self.candidates.first()
    }

    pub fn select_candidate(&mut self, candidate_id: &str) -> Option<&CachedLyricsCandidate> {
        let candidate = self
            .candidates
            .iter()
            .find(|candidate| candidate.id == candidate_id)?;
        self.selected_candidate_id = Some(candidate.id.clone());
        self.provider_id = candidate.provider_id.clone();
        self.synced = candidate.synced;
        self.lines = candidate.lines.clone();
        self.candidates
            .iter()
            .find(|value| value.id == candidate_id)
    }

    pub fn to_candidates(&self) -> Vec<LyricsCandidate> {
        self.candidates
            .iter()
            .map(|candidate| LyricsCandidate {
                id: candidate.id.clone(),
                provider_id: candidate.provider_id.clone(),
                label: candidate.label.clone(),
                synced: candidate.synced,
                preview: candidate.preview.clone(),
                lines: candidate.lines.clone(),
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            })
            .collect()
    }
}

pub fn load_cached_lyrics(app: &AppHandle, signature: &TrackSignature) -> Option<CachedLyricsEntry> {
    read_cached_lyrics_at_path(cache_file_path(app, signature).ok()?)
        .or_else(|| {
            if signature.duration_seconds <= 0.0 {
                return None;
            }
            let mut legacy_signature = signature.clone();
            legacy_signature.duration_seconds = 0.0;
            read_cached_lyrics_at_path(cache_file_path_legacy(app, &legacy_signature).ok()?)
        })
        .or_else(|| {
            if signature.duration_seconds <= 0.0 {
                return None;
            }
            read_cached_lyrics_at_path(cache_file_path_legacy(app, signature).ok()?)
        })
}

fn read_cached_lyrics_at_path(path: PathBuf) -> Option<CachedLyricsEntry> {
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(&path).ok()?;
    let mut entry: CachedLyricsEntry = serde_json::from_str(&raw).ok()?;
    entry.normalize_legacy();
    if entry.candidates.is_empty() {
        return None;
    }
    Some(entry)
}

pub fn save_cached_lyrics(
    app: &AppHandle,
    signature: &TrackSignature,
    entry: &CachedLyricsEntry,
) -> Result<(), String> {
    let path = cache_file_path(app, signature)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create lyrics cache dir failed: {err}"))?;
    }
    let encoded =
        serde_json::to_string_pretty(entry).map_err(|err| format!("encode lyrics cache failed: {err}"))?;
    fs::write(path, encoded).map_err(|err| format!("write lyrics cache failed: {err}"))?;
    Ok(())
}

/// 清除曲目关联的歌词候选缓存（含旧版 duration 键）。
pub fn clear_cached_lyrics(app: &AppHandle, signature: &TrackSignature) -> Result<(), String> {
    let mut legacy_zero = signature.clone();
    legacy_zero.duration_seconds = 0.0;
    for path in [
        cache_file_path(app, signature)?,
        cache_file_path_legacy(app, &legacy_zero)?,
        cache_file_path_legacy(app, signature)?,
    ] {
        if path.exists() {
            fs::remove_file(&path).map_err(|err| format!("remove lyrics cache failed: {err}"))?;
        }
    }
    Ok(())
}

fn signature_cache_key(signature: &TrackSignature) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    signature.track_name.hash(&mut hasher);
    signature.artist_name.hash(&mut hasher);
    signature.album_name.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn signature_cache_key_legacy(signature: &TrackSignature) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    signature.track_name.hash(&mut hasher);
    signature.artist_name.hash(&mut hasher);
    signature.album_name.hash(&mut hasher);
    signature.duration_seconds.to_bits().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn cache_file_path(app: &AppHandle, signature: &TrackSignature) -> Result<PathBuf, String> {
    Ok(lyrics_cache_dir(app)?.join(format!("{}.json", signature_cache_key(signature))))
}

fn cache_file_path_legacy(app: &AppHandle, signature: &TrackSignature) -> Result<PathBuf, String> {
    Ok(lyrics_cache_dir(app)?.join(format!("{}.json", signature_cache_key_legacy(signature))))
}

fn lyrics_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_cache_dir()
        .map_err(|err| format!("resolve app cache dir failed: {err}"))?;
    path.push("lyrics");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_ignores_duration() {
        let base = TrackSignature {
            track_name: "Song".to_string(),
            artist_name: "Artist".to_string(),
            album_name: "Album".to_string(),
            duration_seconds: 0.0,
        };
        let with_duration = TrackSignature {
            duration_seconds: 245.5,
            ..base.clone()
        };
        assert_eq!(signature_cache_key(&base), signature_cache_key(&with_duration));
    }
}
