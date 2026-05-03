use crate::app::windows;

pub fn reveal_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    windows::show_main_window(app)
}

pub fn reveal_preferences_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    windows::show_preferences_window(app)
}
