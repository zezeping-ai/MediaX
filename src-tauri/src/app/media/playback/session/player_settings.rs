use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::state::MediaState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

const PLAYER_SETTINGS_FILE_NAME: &str = "player-settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSettingsFile {
    #[serde(default = "default_resume_last_position")]
    pub resume_last_position: bool,
}

fn default_resume_last_position() -> bool {
    true
}

impl Default for PlayerSettingsFile {
    fn default() -> Self {
        Self {
            resume_last_position: true,
        }
    }
}

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

pub fn bootstrap_player_settings(app: &AppHandle, state: &MediaState) -> MediaResult<()> {
    let path = player_settings_path(app)?;
    let settings = load_player_settings(&path);
    state
        .runtime
        .resume_last_position_enabled
        .store(settings.resume_last_position, Ordering::Relaxed);
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
        .store(enabled, Ordering::Relaxed);
    let path = player_settings_path(app)?;
    save_player_settings(
        &path,
        &PlayerSettingsFile {
            resume_last_position: enabled,
        },
    )
}

pub fn resume_last_position_enabled(state: &MediaState) -> bool {
    state
        .runtime
        .resume_last_position_enabled
        .load(Ordering::Relaxed)
}

pub fn should_persist_playback_progress(state: &MediaState) -> bool {
    resume_last_position_enabled(state)
}
