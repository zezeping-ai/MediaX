use crate::app::media::playback::debug_log::append_playback_debug_log;
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use crate::app::shell::window_actions::reveal_main_window;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
#[cfg(any(desktop, target_os = "macos"))]
use tauri::Url;
#[cfg(desktop)]
use tauri_plugin_deep_link::DeepLinkExt;

const AUTOPROBE_SOURCE_ENV: &str = "MEDIAX_AUTOPROBE_SOURCE";
const MEDIAX_DEEP_LINK_SCHEME: &str = "mediax";

#[derive(Clone, Copy)]
pub struct OpenSourceRequest<'a> {
    pub source: &'a str,
    pub stage: &'static str,
}

pub fn bootstrap_from_cli_args(app: &AppHandle) {
    if let Some(source) = std::env::args_os().skip(1).find_map(normalize_cli_source) {
        dispatch_open_request(
            app,
            OpenSourceRequest {
                source: &source,
                stage: "launch_arg",
            },
        );
    }
}

#[cfg(desktop)]
pub fn bootstrap_from_deep_links(app: &AppHandle) -> Result<(), String> {
    let current_urls = app
        .deep_link()
        .get_current()
        .map_err(|err| format!("read deep link urls failed: {err}"))?;
    if let Some(source) = current_urls
        .as_deref()
        .and_then(|urls| urls.iter().find_map(normalize_mediax_deep_link))
    {
        dispatch_open_request(
            app,
            OpenSourceRequest {
                source: &source,
                stage: "deep_link_start",
            },
        );
    }

    let app_handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        if let Some(source) = event.urls().iter().find_map(normalize_mediax_deep_link) {
            dispatch_open_request(
                &app_handle,
                OpenSourceRequest {
                    source: &source,
                    stage: "deep_link_runtime",
                },
            );
        }
    });
    Ok(())
}

#[cfg(desktop)]
pub fn handle_secondary_launch(app: &AppHandle, args: &[String]) {
    let _ = reveal_main_window(app);
    if let Some(source) = args
        .iter()
        .skip(1)
        .find_map(|arg| normalize_secondary_launch_arg(arg))
    {
        dispatch_open_request(
            app,
            OpenSourceRequest {
                source: &source,
                stage: "single_instance_arg",
            },
        );
    }
}

#[cfg(target_os = "macos")]
pub fn handle_opened_urls(app: &AppHandle, urls: &[Url]) {
    if let Some(source) = urls.iter().find_map(normalize_opened_url) {
        dispatch_open_request(
            app,
            OpenSourceRequest {
                source: &source,
                stage: "opened_event",
            },
        );
    }
}

pub fn dispatch_open_request(app: &AppHandle, request: OpenSourceRequest<'_>) {
    if has_autoprobe_source() {
        return;
    }

    schedule_open_source(app.clone(), request.source.to_string(), request.stage);
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

#[cfg(desktop)]
fn normalize_secondary_launch_arg(raw: &str) -> Option<String> {
    if raw
        .trim()
        .to_ascii_lowercase()
        .starts_with(&format!("{MEDIAX_DEEP_LINK_SCHEME}://"))
    {
        return None;
    }
    normalize_cli_source(OsString::from(raw))
}

#[cfg(target_os = "macos")]
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
        append_shell_log(&app, stage, &format!("open source: {source}"));
        let _ = reveal_main_window(&app);
        if let Err(err) = coordinator::open(app.clone(), app.state::<MediaState>(), source.clone(), None) {
            append_shell_log(&app, "launch_error", &format!("open failed: {err}"));
            return;
        }
        if let Err(err) = coordinator::play(app.clone(), app.state::<MediaState>(), None) {
            append_shell_log(&app, "launch_error", &format!("play failed: {err}"));
        }
    });
}

fn append_shell_log(app: &AppHandle, stage: &str, message: &str) {
    let at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default();
    append_playback_debug_log(app, at_ms, stage, message);
}
