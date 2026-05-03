use crate::app::shell::open_request;
use tauri::AppHandle;
#[cfg(any(desktop, target_os = "macos"))]
use tauri::Url;

pub fn bootstrap_from_launch_sources(app: &AppHandle) {
    open_request::bootstrap_from_cli_args(app);
}

#[cfg(desktop)]
pub fn bootstrap_from_deep_links(app: &AppHandle) -> Result<(), String> {
    open_request::bootstrap_from_deep_links(app)
}

#[cfg(desktop)]
pub fn handle_secondary_launch(app: &AppHandle, args: &[String]) {
    open_request::handle_secondary_launch(app, args);
}

#[cfg(target_os = "macos")]
pub fn handle_opened_urls(app: &AppHandle, urls: &[Url]) {
    open_request::handle_opened_urls(app, urls);
}
