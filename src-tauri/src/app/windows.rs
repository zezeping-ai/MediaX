use crate::app::media::model::PlaybackStatus;
use crate::app::media::playback::render::renderer::{RendererState, VideoScaleMode};
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use std::sync::{Mutex, OnceLock};
use tauri::Manager;
use tauri::{PhysicalPosition, PhysicalSize, Position, Size};
use tauri::State;

const MAIN_WINDOW_LABEL: &str = "main";
const PREFERENCES_WINDOW_LABEL: &str = "preferences";
const VIDEO_TRANSCODE_WINDOW_LABEL: &str = "video_transcode";
const AUDIO_TRANSCODE_WINDOW_LABEL: &str = "audio_transcode";
const IMAGE_COMPRESS_WINDOW_LABEL: &str = "image_compress";

#[derive(Clone, Copy)]
struct WindowRestoreBounds {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

fn main_window_restore_bounds() -> &'static Mutex<Option<WindowRestoreBounds>> {
    static RESTORE_BOUNDS: OnceLock<Mutex<Option<WindowRestoreBounds>>> = OnceLock::new();
    RESTORE_BOUNDS.get_or_init(|| Mutex::new(None))
}

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

pub fn show_video_transcode_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(VIDEO_TRANSCODE_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        VIDEO_TRANSCODE_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/tools/video-transcode".into()),
    )
    .title("视频转码")
    .inner_size(980.0, 720.0)
    .min_inner_size(860.0, 640.0)
    .resizable(true)
    .visible(true)
    .build()?;
    Ok(())
}

pub fn show_audio_transcode_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(AUDIO_TRANSCODE_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        AUDIO_TRANSCODE_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/tools/audio-transcode".into()),
    )
    .title("音频转码")
    .inner_size(980.0, 720.0)
    .min_inner_size(860.0, 640.0)
    .resizable(true)
    .visible(true)
    .build()?;
    Ok(())
}

pub fn show_image_compress_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(IMAGE_COMPRESS_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        IMAGE_COMPRESS_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/tools/image-compress".into()),
    )
    .title("图片压缩")
    .inner_size(1040.0, 760.0)
    .min_inner_size(920.0, 680.0)
    .resizable(true)
    .visible(true)
    .build()?;
    Ok(())
}

pub fn handle_close_requested(window: &tauri::Window, event: &tauri::WindowEvent) {
    // 点击窗口关闭按钮时：隐藏窗口，不退出应用（托盘仍可恢复）
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() != MAIN_WINDOW_LABEL {
            return;
        }
        // If the user closes the main window while playing, pause playback first to avoid
        // the decode thread continuing in background unexpectedly.
        if window.label() == MAIN_WINDOW_LABEL {
            let app = window.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state = app.state::<MediaState>();
                let is_playing = state
                    .session
                    .playback
                    .lock()
                    .ok()
                    .map(|playback| playback.state().status == PlaybackStatus::Playing)
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
        let position = window
            .outer_position()
            .map_err(|err| format!("read window position failed: {err}"))?;
        let size = window
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

#[tauri::command]
pub fn window_reveal_file(path: String) -> Result<(), String> {
    let target = Path::new(path.trim());
    if !target.exists() {
        return Err("文件不存在".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(target)
            .status()
            .map_err(|err| format!("打开文件位置失败: {err}"))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg("/select,")
            .arg(target)
            .status()
            .map_err(|err| format!("打开文件位置失败: {err}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let dir = target.parent().unwrap_or(target);
        Command::new("xdg-open")
            .arg(dir)
            .status()
            .map_err(|err| format!("打开文件位置失败: {err}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("当前平台不支持该操作".to_string())
}
