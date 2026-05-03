use tauri::{AppHandle, Emitter};

pub const FRONTEND_SHELL_EVENT: &str = "media://menu-action";

pub fn emit_open_local_request(app: &AppHandle) {
    let _ = app.emit(FRONTEND_SHELL_EVENT, "open_local");
}

pub fn emit_open_url_request(app: &AppHandle) {
    let _ = app.emit(FRONTEND_SHELL_EVENT, "open_url");
}
