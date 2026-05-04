use crate::app::media::playback::dto::PlaybackChannelRouting;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn parse_duration_ms_env(key: &str, fallback: Duration) -> Duration {
    let Some(raw) = std::env::var(key).ok() else {
        return fallback;
    };
    let Ok(ms) = raw.trim().parse::<u64>() else {
        return fallback;
    };
    Duration::from_millis(ms)
}

pub(super) fn collect_media_sources_from_dir(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|err| format!("read dir `{}` failed: {err}", dir.display()))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| path.is_file() && is_supported_media_path(path, extensions))
        .collect();
    paths.sort();
    Ok(paths
        .into_iter()
        .filter_map(|path| path.to_str().map(ToString::to_string))
        .collect())
}

fn is_supported_media_path(path: &Path, extensions: &[&str]) -> bool {
    let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };
    extensions.iter().any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

pub(super) fn parse_bool(value: &str) -> Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err("expected boolean".to_string()),
    }
}

pub(super) fn parse_routing(value: &str) -> Result<PlaybackChannelRouting, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "stereo" => Ok(PlaybackChannelRouting::Stereo),
        "left_to_both" => Ok(PlaybackChannelRouting::LeftToBoth),
        "right_to_both" => Ok(PlaybackChannelRouting::RightToBoth),
        _ => Err("expected stereo|left_to_both|right_to_both".to_string()),
    }
}

pub(super) fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

