use crate::app::media::error::{MediaError, MediaResult};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

const PLAYBACK_DEBUG_LOG_FILE_NAME: &str = "playback-debug.log";
const PLAYBACK_DEBUG_LOG_ROTATED_FILE_NAME: &str = "playback-debug.1.log";
const PLAYBACK_DEBUG_LOG_MAX_BYTES: u64 = 1024 * 1024;
static PLAYBACK_DEBUG_WRITER: OnceLock<Mutex<PlaybackDebugLogWriter>> = OnceLock::new();

pub(crate) fn append_playback_debug_log(app: &AppHandle, at_ms: u64, stage: &str, message: &str) {
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
    let writer = PLAYBACK_DEBUG_WRITER.get_or_init(|| Mutex::new(PlaybackDebugLogWriter::default()));
    let Ok(mut writer) = writer.lock() else {
        return;
    };
    let line = format!("[{at_ms}] {stage}: {message}\n");
    let _ = writer.append_line(&log_path, &rotated_log_path, &line);
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
    if let Some(writer) = PLAYBACK_DEBUG_WRITER.get() {
        if let Ok(mut writer) = writer.lock() {
            writer.reset();
        }
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

#[derive(Default)]
struct PlaybackDebugLogWriter {
    path: Option<PathBuf>,
    rotated_path: Option<PathBuf>,
    file: Option<File>,
    bytes_written: u64,
}

impl PlaybackDebugLogWriter {
    fn reset(&mut self) {
        self.path = None;
        self.rotated_path = None;
        self.file = None;
        self.bytes_written = 0;
    }

    fn append_line(
        &mut self,
        log_path: &PathBuf,
        rotated_log_path: &PathBuf,
        line: &str,
    ) -> std::io::Result<()> {
        self.ensure_open(log_path, rotated_log_path)?;
        let projected = self.bytes_written.saturating_add(line.len() as u64);
        if projected >= PLAYBACK_DEBUG_LOG_MAX_BYTES {
            self.rotate_current_file()?;
            self.ensure_open(log_path, rotated_log_path)?;
        }
        if let Some(file) = self.file.as_mut() {
            file.write_all(line.as_bytes())?;
            self.bytes_written = self.bytes_written.saturating_add(line.len() as u64);
        }
        Ok(())
    }

    fn ensure_open(&mut self, log_path: &PathBuf, rotated_log_path: &PathBuf) -> std::io::Result<()> {
        let switched_target = self.path.as_ref() != Some(log_path)
            || self.rotated_path.as_ref() != Some(rotated_log_path);
        if switched_target || self.file.is_none() {
            self.path = Some(log_path.clone());
            self.rotated_path = Some(rotated_log_path.clone());
            self.file = Some(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_path)?,
            );
            self.bytes_written = fs::metadata(log_path).map(|meta| meta.len()).unwrap_or(0);
        }
        Ok(())
    }

    fn rotate_current_file(&mut self) -> std::io::Result<()> {
        let Some(log_path) = self.path.clone() else {
            return Ok(());
        };
        let Some(rotated_path) = self.rotated_path.clone() else {
            return Ok(());
        };
        self.file = None;
        rotate_playback_debug_log_if_needed(&log_path, &rotated_path);
        self.bytes_written = 0;
        Ok(())
    }
}
