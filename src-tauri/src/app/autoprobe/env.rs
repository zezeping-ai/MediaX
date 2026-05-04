use crate::app::media::playback::debug_log::append_playback_debug_log;
use std::path::Path;
use tauri::AppHandle;

use super::util::{collect_media_sources_from_dir, now_unix_ms};

pub(super) fn resolve_sources_from_env(
    app: &AppHandle,
    single_key: &str,
    list_key: &str,
    dir_key: &str,
    media_extensions: &[&str],
) -> Result<Vec<String>, String> {
    let mut sources = Vec::new();

    if let Some(source) = std::env::var(single_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        sources.push(source);
    }

    if let Some(raw_list) = std::env::var(list_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        sources.extend(
            raw_list
                .split(';')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        );
    }

    if let Some(dir) = std::env::var(dir_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let mut from_dir = collect_media_sources_from_dir(Path::new(&dir), media_extensions)?;
        sources.append(&mut from_dir);
    }

    sources.sort();
    sources.dedup();

    if !sources.is_empty() {
        append_playback_debug_log(
            app,
            now_unix_ms(),
            "autoprobe",
            &format!("resolved {} source(s) from env", sources.len()),
        );
    }

    Ok(sources)
}

