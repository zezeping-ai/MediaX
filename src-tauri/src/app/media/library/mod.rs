use crate::app::media::error::MediaError;
use crate::app::media::model::{MediaItem, MediaLibraryState, MediaSnapshot};
use crate::app::media::state::{emit_snapshot, MediaState};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use walkdir::WalkDir;

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v", "wmv", "mpeg", "mpg", "ts", "m2ts",
];

#[derive(Default)]
pub struct MediaLibraryService {
    state: MediaLibraryState,
    recent_progress: HashMap<String, (u64, f64)>,
}

impl MediaLibraryService {
    pub fn state(&self) -> MediaLibraryState {
        self.state.clone()
    }

    pub fn set_roots_and_scan(&mut self, roots: Vec<String>) {
        self.state.roots = roots;
        self.rescan();
    }

    pub fn rescan(&mut self) {
        let mut items = Vec::new();
        for root in &self.state.roots {
            let root_path = PathBuf::from(root);
            if !root_path.exists() || !root_path.is_dir() {
                continue;
            }
            self.collect_items_from_root(&root_path, &mut items);
        }
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.state.items = items;
    }

    pub fn mark_playback_progress(&mut self, path: &str, position_seconds: f64) {
        let now = now_epoch_seconds();
        self.recent_progress
            .insert(path.to_string(), (now, position_seconds));
        if let Some(item) = self.state.items.iter_mut().find(|item| item.path == path) {
            item.last_played_at = Some(now);
            item.last_position_seconds = position_seconds.max(0.0);
        }
    }

    fn collect_items_from_root(&self, root_path: &Path, items: &mut Vec<MediaItem>) {
        for entry in WalkDir::new(root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_supported_media_file(path) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let path_string = path.to_string_lossy().to_string();
            let (last_played_at, last_position_seconds) = self
                .recent_progress
                .get(&path_string)
                .map_or((None, 0.0), |(played_at, position)| {
                    (Some(*played_at), *position)
                });
            items.push(MediaItem {
                id: path_string.clone(),
                path: path_string,
                name: entry.file_name().to_string_lossy().to_string(),
                extension: path
                    .extension()
                    .map_or_else(String::new, |ext| ext.to_string_lossy().to_lowercase()),
                size_bytes: metadata.len(),
                last_played_at,
                last_position_seconds,
            });
        }
    }
}

#[tauri::command]
pub fn media_set_library_roots(
    app: AppHandle,
    state: State<'_, MediaState>,
    roots: Vec<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?;
        library.set_roots_and_scan(roots);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_rescan_library(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?;
        library.rescan();
    }
    emit_snapshot(&app, &state)
}

fn is_supported_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
