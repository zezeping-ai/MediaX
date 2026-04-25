use crate::app::media::player::coordinator;
use crate::app::media::player::renderer::{RendererState, VideoScaleMode};
use crate::app::media::player::state::MediaState;
use crate::app::media::types::PlaybackStatus;
use tauri::Manager;
use tauri::State;

const MAIN_WINDOW_LABEL: &str = "main";
const PREFERENCES_WINDOW_LABEL: &str = "preferences";

pub fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub fn show_preferences_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(PREFERENCES_WINDOW_LABEL) {
        window.show()?;
        // Keep preferences above the main window (even if main is always-on-top).
        let _ = window.set_always_on_top(true);
        window.set_focus()?;
        return Ok(());
    }

    // 使用 hash 路由，避免生产环境 file:// 下的 history 路由问题
    let window = tauri::WebviewWindowBuilder::new(
        app,
        PREFERENCES_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/preferences".into()),
    )
    .title("系统设置")
    // Default larger so sections fit without feeling cramped.
    .inner_size(820.0, 620.0)
    .min_inner_size(720.0, 520.0)
    .resizable(true)
    .visible(true)
    .build()?;

    // Preferences should stay on top of the player window.
    let _ = window.set_always_on_top(true);

    Ok(())
}

pub fn handle_close_requested(window: &tauri::Window, event: &tauri::WindowEvent) {
    // 点击窗口关闭按钮时：隐藏窗口，不退出应用（托盘仍可恢复）
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        // If the user closes the main window while playing, pause playback first to avoid
        // the decode thread continuing in background unexpectedly.
        if window.label() == MAIN_WINDOW_LABEL {
            let app = window.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = app.state::<MediaState>();
                let is_playing = state
                    .playback
                    .lock()
                    .ok()
                    .map(|mut playback| playback.state().status == PlaybackStatus::Playing)
                    .unwrap_or(false);
                if is_playing {
                    let _ = coordinator::pause(app.clone(), state, None);
                }
            });
        }
        api.prevent_close();
        let _ = window.hide();
    }
}

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
