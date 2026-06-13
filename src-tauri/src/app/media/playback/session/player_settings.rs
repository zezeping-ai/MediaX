use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::state::MediaState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use tauri::{AppHandle, Manager};

const PLAYER_SETTINGS_FILE_NAME: &str = "player-settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsProviderSettings {
    #[serde(default = "default_true")]
    pub lrclib: bool,
    #[serde(default = "default_true")]
    pub lrcapi: bool,
    #[serde(default = "default_true")]
    pub kugou: bool,
    #[serde(default = "default_true")]
    pub netease: bool,
}

impl Default for LyricsProviderSettings {
    fn default() -> Self {
        Self {
            lrclib: true,
            lrcapi: true,
            kugou: true,
            netease: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsFetchSettings {
    #[serde(default = "default_true")]
    pub auto_fetch_online_lyrics: bool,
    #[serde(default)]
    pub providers: LyricsProviderSettings,
    #[serde(default)]
    pub lrc_api_base_url: String,
}

impl Default for LyricsFetchSettings {
    fn default() -> Self {
        Self {
            auto_fetch_online_lyrics: true,
            providers: LyricsProviderSettings::default(),
            lrc_api_base_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSettingsFile {
    #[serde(default = "default_resume_last_position")]
    pub resume_last_position: bool,
    #[serde(default)]
    pub lyrics: LyricsFetchSettings,
}

fn default_resume_last_position() -> bool {
    true
}

fn default_true() -> bool {
    true
}

impl Default for PlayerSettingsFile {
    fn default() -> Self {
        Self {
            resume_last_position: true,
            lyrics: LyricsFetchSettings::default(),
        }
    }
}

static LYRICS_FETCH_SETTINGS: OnceLock<RwLock<LyricsFetchSettings>> = OnceLock::new();

pub fn player_settings_path(app: &AppHandle) -> MediaResult<PathBuf> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|err| MediaError::internal(format!("resolve app data dir failed: {err}")))?;
    path.push(PLAYER_SETTINGS_FILE_NAME);
    Ok(path)
}

pub fn load_player_settings(path: &Path) -> PlayerSettingsFile {
    let Ok(raw) = fs::read_to_string(path) else {
        return PlayerSettingsFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_player_settings(path: &Path, settings: &PlayerSettingsFile) -> MediaResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| MediaError::internal(format!("create player settings dir failed: {err}")))?;
    }
    let encoded = serde_json::to_string_pretty(settings)
        .map_err(|err| MediaError::internal(format!("encode player settings failed: {err}")))?;
    fs::write(path, encoded)
        .map_err(|err| MediaError::internal(format!("write player settings failed: {err}")))?;
    Ok(())
}

fn lyrics_settings_store() -> &'static RwLock<LyricsFetchSettings> {
    LYRICS_FETCH_SETTINGS.get_or_init(|| RwLock::new(LyricsFetchSettings::default()))
}

pub fn bootstrap_player_settings(app: &AppHandle, state: &MediaState) -> MediaResult<()> {
    let path = player_settings_path(app)?;
    let settings = load_player_settings(&path);
    state
        .runtime
        .resume_last_position_enabled
        .store(settings.resume_last_position, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut store) = lyrics_settings_store().write() {
        *store = settings.lyrics.clone();
    }
    Ok(())
}

pub fn set_resume_last_position(
    app: &AppHandle,
    state: &MediaState,
    enabled: bool,
) -> MediaResult<()> {
    state
        .runtime
        .resume_last_position_enabled
        .store(enabled, std::sync::atomic::Ordering::Relaxed);
    let path = player_settings_path(app)?;
    let mut settings = load_player_settings(&path);
    settings.resume_last_position = enabled;
    save_player_settings(&path, &settings)
}

pub fn set_lyrics_fetch_settings(app: &AppHandle, lyrics: LyricsFetchSettings) -> MediaResult<()> {
    if let Ok(mut store) = lyrics_settings_store().write() {
        *store = lyrics.clone();
    }
    let path = player_settings_path(app)?;
    let mut settings = load_player_settings(&path);
    settings.lyrics = lyrics;
    save_player_settings(&path, &settings)
}

pub fn lyrics_fetch_settings() -> LyricsFetchSettings {
    lyrics_settings_store()
        .read()
        .map(|value| value.clone())
        .unwrap_or_default()
}

pub fn resume_last_position_enabled(state: &MediaState) -> bool {
    state
        .runtime
        .resume_last_position_enabled
        .load(std::sync::atomic::Ordering::Relaxed)
}

pub fn should_persist_playback_progress(state: &MediaState) -> bool {
    resume_last_position_enabled(state)
}
