use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const LYRICS_SELECTIONS_FILE: &str = "lyrics-selections.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LyricsSelectionsFile {
    #[serde(default)]
    selections: HashMap<String, String>,
}

pub fn load_selected_candidate_id(app: &AppHandle, source_path: &str) -> Option<String> {
    let path = selections_file_path(app).ok()?;
    if !path.exists() {
        return None;
    }
    let raw = fs::read_to_string(path).ok()?;
    let file: LyricsSelectionsFile = serde_json::from_str(&raw).ok()?;
    file.selections.get(source_path).cloned()
}

pub fn save_selected_candidate_id(
    app: &AppHandle,
    source_path: &str,
    candidate_id: &str,
) -> Result<(), String> {
    let path = selections_file_path(app)?;
    let mut file = if path.exists() {
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("read lyrics selections failed: {err}"))?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        LyricsSelectionsFile::default()
    };
    file.selections
        .insert(source_path.to_string(), candidate_id.to_string());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create lyrics selections dir failed: {err}"))?;
    }
    let encoded = serde_json::to_string_pretty(&file)
        .map_err(|err| format!("encode lyrics selections failed: {err}"))?;
    fs::write(path, encoded).map_err(|err| format!("write lyrics selections failed: {err}"))?;
    Ok(())
}

pub fn clear_selected_candidate_id(app: &AppHandle, source_path: &str) -> Result<(), String> {
    let path = selections_file_path(app)?;
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("read lyrics selections failed: {err}"))?;
    let mut file: LyricsSelectionsFile = serde_json::from_str(&raw).unwrap_or_default();
    if file.selections.remove(source_path).is_none() {
        return Ok(());
    }
    if file.selections.is_empty() {
        fs::remove_file(&path).map_err(|err| format!("remove lyrics selections failed: {err}"))?;
        return Ok(());
    }
    let encoded = serde_json::to_string_pretty(&file)
        .map_err(|err| format!("encode lyrics selections failed: {err}"))?;
    fs::write(path, encoded).map_err(|err| format!("write lyrics selections failed: {err}"))?;
    Ok(())
}

fn selections_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("resolve app data dir failed: {err}"))?;
    path.push(LYRICS_SELECTIONS_FILE);
    Ok(path)
}
