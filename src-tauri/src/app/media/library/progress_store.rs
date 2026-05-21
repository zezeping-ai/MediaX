use crate::app::media::error::{MediaError, MediaResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

pub const PLAYBACK_PROGRESS_FILE_NAME: &str = "playback-progress.json";
const RESUME_TAIL_EPSILON_SECONDS: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub struct ProgressRecord {
    pub played_at: u64,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedProgressEntry {
    played_at: u64,
    position_seconds: f64,
    #[serde(default)]
    duration_seconds: Option<f64>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedProgressFile {
    entries: HashMap<String, PersistedProgressEntry>,
}

pub fn playback_progress_path(app: &AppHandle) -> MediaResult<PathBuf> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|err| MediaError::internal(format!("resolve app data dir failed: {err}")))?;
    path.push(PLAYBACK_PROGRESS_FILE_NAME);
    Ok(path)
}

pub fn load_progress_map(path: &Path) -> HashMap<String, ProgressRecord> {
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let parsed = serde_json::from_str::<PersistedProgressFile>(&raw).unwrap_or_default();
    parsed
        .entries
        .into_iter()
        .map(|(path, entry)| {
            (
                path,
                ProgressRecord {
                    played_at: entry.played_at,
                    position_seconds: entry.position_seconds.max(0.0),
                    duration_seconds: entry.duration_seconds,
                },
            )
        })
        .collect()
}

pub fn save_progress_map(path: &Path, entries: &HashMap<String, ProgressRecord>) -> MediaResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| MediaError::internal(format!("create progress dir failed: {err}")))?;
    }
    let payload = PersistedProgressFile {
        entries: entries
            .iter()
            .map(|(path, record)| {
                (
                    path.clone(),
                    PersistedProgressEntry {
                        played_at: record.played_at,
                        position_seconds: record.position_seconds,
                        duration_seconds: record.duration_seconds,
                    },
                )
            })
            .collect(),
    };
    let encoded = serde_json::to_string_pretty(&payload)
        .map_err(|err| MediaError::internal(format!("encode progress failed: {err}")))?;
    fs::write(path, encoded)
        .map_err(|err| MediaError::internal(format!("write progress failed: {err}")))?;
    Ok(())
}

pub fn normalize_saved_position(position_seconds: f64, duration_seconds: Option<f64>) -> f64 {
    let position_seconds = position_seconds.max(0.0);
    let Some(duration_seconds) = duration_seconds.filter(|duration| *duration > 0.0) else {
        return position_seconds;
    };
    if position_seconds + RESUME_TAIL_EPSILON_SECONDS >= duration_seconds {
        return 0.0;
    }
    position_seconds
}
