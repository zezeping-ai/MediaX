use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use crate::app::windows::show_main_window;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Url};
#[cfg(desktop)]
use tauri_plugin_deep_link::DeepLinkExt;

const AUTOPROBE_SOURCE_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE";
const MEDIAX_DEEP_LINK_SCHEME: &str = "mediax";

pub fn bootstrap_from_launch_sources(app: &AppHandle) {
    if has_autoprobe_source() {
        return;
    }

    if let Some(source) = std::env::args_os().skip(1).find_map(normalize_cli_source) {
        schedule_open_source(app.clone(), source, "launch_arg");
    }
}

#[cfg(desktop)]
pub fn bootstrap_from_deep_links(app: &AppHandle) -> Result<(), String> {
    if has_autoprobe_source() {
        return Ok(());
    }

    let current_urls = app
        .deep_link()
        .get_current()
        .map_err(|err| format!("read deep link urls failed: {err}"))?;
    if let Some(source) = current_urls
        .as_deref()
        .and_then(|urls| urls.iter().find_map(normalize_mediax_deep_link))
    {
        schedule_open_source(app.clone(), source, "deep_link_start");
    }

    let app_handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        if let Some(source) = event.urls().iter().find_map(normalize_mediax_deep_link) {
            schedule_open_source(app_handle.clone(), source, "deep_link_runtime");
        }
    });
    Ok(())
}

#[cfg(desktop)]
pub fn handle_secondary_launch(app: &AppHandle, args: &[String]) {
    let _ = show_main_window(app);
    if has_autoprobe_source() {
        return;
    }

    if let Some(source) = args.iter().skip(1).find_map(|arg| {
        if arg
            .trim()
            .to_ascii_lowercase()
            .starts_with(&format!("{MEDIAX_DEEP_LINK_SCHEME}://"))
        {
            return None;
        }
        normalize_cli_source(OsString::from(arg))
    }) {
        schedule_open_source(app.clone(), source, "single_instance_arg");
    }
}

pub fn handle_opened_urls(app: &AppHandle, urls: &[Url]) {
    if has_autoprobe_source() {
        return;
    }

    if let Some(source) = urls.iter().find_map(normalize_opened_url) {
        schedule_open_source(app.clone(), source, "opened_event");
    }
}

fn has_autoprobe_source() -> bool {
    std::env::var(AUTOPROBE_SOURCE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .is_some_and(|value| !value.is_empty())
}

fn normalize_cli_source(raw: OsString) -> Option<String> {
    let raw_path = PathBuf::from(&raw);
    if raw_path.exists() {
        return Some(normalize_path(raw_path));
    }

    let raw_text = raw.to_string_lossy().trim().to_string();
    if raw_text.is_empty() {
        return None;
    }
    if is_supported_remote_source(&raw_text) {
        return Some(raw_text);
    }

    let path = Path::new(&raw_text);
    path.exists().then(|| normalize_path(path.to_path_buf()))
}

fn normalize_opened_url(url: &Url) -> Option<String> {
    match url.scheme() {
        "file" => url.to_file_path().ok().map(normalize_path),
        _ if is_supported_remote_source(url.as_str()) => Some(url.as_str().to_string()),
        _ => None,
    }
}

fn normalize_path(path: PathBuf) -> String {
    path.canonicalize()
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn normalize_mediax_deep_link(url: &Url) -> Option<String> {
    if url.scheme() != MEDIAX_DEEP_LINK_SCHEME {
        return None;
    }

    let action = url.host_str().map(|value| value.to_ascii_lowercase());
    if action
        .as_deref()
        .is_some_and(|value| !matches!(value, "open" | "play"))
    {
        return None;
    }

    let deep_link_value = url
        .query_pairs()
        .find_map(|(key, value)| match key.as_ref() {
            "url" | "source" | "path" => Some(value.into_owned()),
            _ => None,
        })?;

    if deep_link_value.is_empty() {
        return None;
    }

    if is_supported_remote_source(&deep_link_value) {
        return Some(deep_link_value);
    }

    let path = PathBuf::from(&deep_link_value);
    path.exists().then(|| normalize_path(path))
}

fn is_supported_remote_source(source: &str) -> bool {
    matches!(
        source.split(':').next().map(|value| value.to_ascii_lowercase()),
        Some(scheme)
            if matches!(
                scheme.as_str(),
                "http" | "https" | "rtsp" | "rtmp" | "mms" | "file"
            )
    )
}

fn schedule_open_source(app: AppHandle, source: String, stage: &'static str) {
    tauri::async_runtime::spawn(async move {
        std::thread::sleep(Duration::from_millis(320));
        append_launch_log(&app, stage, &format!("open source: {source}"));
        let _ = show_main_window(&app);
        if let Err(err) = coordinator::open(app.clone(), app.state::<MediaState>(), source.clone(), None) {
            append_launch_log(&app, "launch_error", &format!("open failed: {err}"));
            return;
        }
        if let Err(err) = coordinator::play(app.clone(), app.state::<MediaState>(), None) {
            append_launch_log(&app, "launch_error", &format!("play failed: {err}"));
        }
    });
}

fn append_launch_log(app: &AppHandle, stage: &str, message: &str) {
    let at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();
    append_playback_debug_log(app, at_ms, stage, message);
}
