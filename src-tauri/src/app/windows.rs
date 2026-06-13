use crate::app::media::model::PlaybackStatus;
use crate::app::media::playback::render::renderer::{RendererState, VideoScaleMode};
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri::State;
use tauri::{PhysicalPosition, PhysicalSize, Position, Size};

const MAIN_WINDOW_LABEL: &str = "main";
const DEFAULT_MAIN_WINDOW_TITLE: &str = env!("CARGO_PKG_NAME");
const PREFERENCES_WINDOW_LABEL: &str = "preferences";
const VIDEO_TRANSCODE_WINDOW_LABEL: &str = "video_transcode";
const AUDIO_TRANSCODE_WINDOW_LABEL: &str = "audio_transcode";
const IMAGE_COMPRESS_WINDOW_LABEL: &str = "image_compress";
const USER_FEEDBACK_WINDOW_LABEL: &str = "user_feedback";

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

fn restore_main_window_bounds_after_fullscreen(window: tauri::WebviewWindow) {
    let restore_bounds = main_window_restore_bounds()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take());
    let Some(bounds) = restore_bounds else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        thread::sleep(Duration::from_millis(140));
        if bounds.maximized {
            let _ = window.maximize();
            return;
        }
        let _ = window.unmaximize();
        let _ = window.set_size(Size::Physical(bounds.size));
        let _ = window.set_position(Position::Physical(bounds.position));
    });
}

fn main_window_is_fullscreen(app: &tauri::AppHandle) -> bool {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .and_then(|window| window.is_fullscreen().ok())
        .unwrap_or(false)
}

fn request_exit_main_window_fullscreen(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.set_fullscreen(false);
    }
}

async fn wait_for_main_window_fullscreen_exit(app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    const MAX_WAIT: Duration = Duration::from_secs(3);
    #[cfg(not(target_os = "macos"))]
    const MAX_WAIT: Duration = Duration::from_millis(500);

    let deadline = Instant::now() + MAX_WAIT;
    while Instant::now() < deadline {
        if !main_window_is_fullscreen(app) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    // macOS 全屏退出动画未完成时 hide 会失效（tauri#10580 / #12056）。
    #[cfg(target_os = "macos")]
    thread::sleep(Duration::from_millis(350));
}

async fn hide_main_window_safely(app: tauri::AppHandle) {
    if main_window_is_fullscreen(&app) {
        request_exit_main_window_fullscreen(&app);
        wait_for_main_window_fullscreen_exit(&app).await;
    }

    for _ in 0..10 {
        let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
            return;
        };
        let _ = window.hide();
        if !window.is_visible().unwrap_or(false) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

pub fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if window.is_fullscreen().unwrap_or(false) {
            let _ = window.set_fullscreen(false);
            restore_main_window_bounds_after_fullscreen(window.clone());
        }
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_set_main_title(app: tauri::AppHandle, title: Option<String>) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window not found".to_string())?;
    let next_title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_MAIN_WINDOW_TITLE);
    window
        .set_title(next_title)
        .map_err(|err| format!("set main title failed: {err}"))?;
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

pub fn show_user_feedback_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(USER_FEEDBACK_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(
        app,
        USER_FEEDBACK_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/feedback".into()),
    )
    .title("用户反馈")
    .inner_size(760.0, 620.0)
    .min_inner_size(680.0, 560.0)
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
        let app = window.app_handle().clone();
        tauri::async_runtime::spawn(async move {
            hide_main_window_safely(app).await;
        });
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
pub fn window_set_main_video_picture_tune(
    renderer: State<'_, RendererState>,
    brightness: i32,
    contrast: i32,
    saturation: i32,
    gamma: i32,
    hue: i32,
) -> Result<(), String> {
    renderer.set_picture_tune(crate::app::media::playback::render::picture_tune::VideoPictureTune::from_ui_values(
        brightness,
        contrast,
        saturation,
        gamma,
        hue,
    ));
    Ok(())
}

#[tauri::command]
pub fn window_set_renderer_backdrop_theme(
    renderer: State<'_, RendererState>,
    theme: String,
) -> Result<(), String> {
    renderer.set_backdrop_theme(theme.as_str());
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
        restore_main_window_bounds_after_fullscreen(window);
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
