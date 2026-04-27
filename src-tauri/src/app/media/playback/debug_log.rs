use crate::app::media::error::{MediaError, MediaResult};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const PLAYBACK_DEBUG_LOG_FILE_NAME: &str = "playback-debug.log";
const PLAYBACK_DEBUG_LOG_ROTATED_FILE_NAME: &str = "playback-debug.1.log";
const PLAYBACK_DEBUG_LOG_MAX_BYTES: u64 = 1024 * 1024;

pub(crate) fn append_playback_debug_log(
    app: &AppHandle,
    at_ms: u64,
    stage: &str,
    message: &str,
) {
    let Ok(log_path) = playback_debug_log_path(app) else {
        return;
    };
    let Ok(rotated_log_path) = playback_debug_log_rotated_path(app) else {
        return;
    };
    let Some(parent_dir) = log_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent_dir).is_err() {
        return;
    }
    rotate_playback_debug_log_if_needed(&log_path, &rotated_log_path);
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    else {
        return;
    };
    let _ = writeln!(file, "[{at_ms}] {stage}: {message}");
}

pub(crate) fn playback_debug_log_path(app: &AppHandle) -> MediaResult<PathBuf> {
    let mut path = app
        .path()
        .app_log_dir()
        .map_err(|err| MediaError::internal(format!("resolve app log dir failed: {err}")))?;
    path.push(PLAYBACK_DEBUG_LOG_FILE_NAME);
    Ok(path)
}

pub(crate) fn playback_debug_log_rotated_path(app: &AppHandle) -> MediaResult<PathBuf> {
    let mut path = app
        .path()
        .app_log_dir()
        .map_err(|err| MediaError::internal(format!("resolve app log dir failed: {err}")))?;
    path.push(PLAYBACK_DEBUG_LOG_ROTATED_FILE_NAME);
    Ok(path)
}

pub(crate) fn clear_playback_debug_log(app: &AppHandle) -> MediaResult<String> {
    let log_path = playback_debug_log_path(app)?;
    let rotated_log_path = playback_debug_log_rotated_path(app)?;
    if let Some(parent_dir) = log_path.parent() {
        fs::create_dir_all(parent_dir)
            .map_err(|err| MediaError::internal(format!("create debug log dir failed: {err}")))?;
    }
    fs::write(&log_path, "")
        .map_err(|err| MediaError::internal(format!("clear debug log failed: {err}")))?;
    if rotated_log_path.exists() {
        fs::remove_file(&rotated_log_path).map_err(|err| {
            MediaError::internal(format!("remove rotated debug log failed: {err}"))
        })?;
    }
    Ok(log_path.display().to_string())
}

pub(crate) fn initialize_playback_debug_log(app: &AppHandle) -> MediaResult<()> {
    clear_playback_debug_log(app).map(|_| ())
}

fn rotate_playback_debug_log_if_needed(log_path: &PathBuf, rotated_log_path: &PathBuf) {
    let Ok(metadata) = fs::metadata(log_path) else {
        return;
    };
    if metadata.len() < PLAYBACK_DEBUG_LOG_MAX_BYTES {
        return;
    }
    if rotated_log_path.exists() && fs::remove_file(rotated_log_path).is_err() {
        return;
    }
    let _ = fs::rename(log_path, rotated_log_path);
}
