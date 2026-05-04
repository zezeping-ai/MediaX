use super::shared::{main_window_restore_bounds, WindowRestoreBounds, MAIN_WINDOW_LABEL};
use crate::app::media::playback::render::renderer::{RendererState, VideoScaleMode};
use std::thread;
use std::time::Duration;
use tauri::Manager;
use tauri::{PhysicalPosition, PhysicalSize, Position, Size, State};

#[tauri::command]
pub fn window_set_main_always_on_top(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;
    window
        .set_always_on_top(enabled)
        .map_err(|err| format!("set always on top failed: {err}"))?;
    Ok(())
}

#[tauri::command]
pub fn window_set_main_video_scale_mode(
    renderer: State<'_, RendererState>,
    mode: String,
) -> Result<(), String> {
    let scale_mode = VideoScaleMode::try_from(mode.as_str())?;
    renderer.set_video_scale_mode(scale_mode);
    Ok(())
}

#[tauri::command]
pub fn window_toggle_main_fullscreen(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;
    let is_fullscreen = window
        .is_fullscreen()
        .map_err(|err| format!("read fullscreen state failed: {err}"))?;
    let next = !is_fullscreen;
    if next {
        let position: PhysicalPosition<i32> = window
            .outer_position()
            .map_err(|err| format!("read window position failed: {err}"))?;
        let size: PhysicalSize<u32> = window
            .outer_size()
            .map_err(|err| format!("read window size failed: {err}"))?;
        let maximized = window
            .is_maximized()
            .map_err(|err| format!("read window maximized state failed: {err}"))?;
        if let Ok(mut guard) = main_window_restore_bounds().lock() {
            *guard = Some(WindowRestoreBounds {
                position,
                size,
                maximized,
            });
        }
    }
    window
        .set_fullscreen(next)
        .map_err(|err| format!("set fullscreen failed: {err}"))?;
    if !next {
        let restore_bounds = main_window_restore_bounds()
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        if let Some(bounds) = restore_bounds {
            let restore_window = window.clone();
            tauri::async_runtime::spawn(async move {
                thread::sleep(Duration::from_millis(140));
                if bounds.maximized {
                    let _ = restore_window.maximize();
                    return;
                }
                let _ = restore_window.unmaximize();
                let _ = restore_window.set_size(Size::Physical(bounds.size));
                let _ = restore_window.set_position(Position::Physical(bounds.position));
            });
        }
    }
    Ok(next)
}

#[tauri::command]
pub fn window_start_main_dragging(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;
    window
        .start_dragging()
        .map_err(|err| format!("start dragging failed: {err}"))?;
    Ok(())
}
