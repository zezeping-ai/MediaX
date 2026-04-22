// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::Manager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

const TRAY_ID: &str = "mediax.tray";
const MENU_PREFERENCES_ID: &str = "mediax.menu.preferences";
const MENU_QUIT_ID: &str = "mediax.menu.quit";
const MAIN_WINDOW_LABEL: &str = "main";
const PREFERENCES_WINDOW_LABEL: &str = "preferences";

fn show_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn show_preferences_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(PREFERENCES_WINDOW_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    // 使用 hash 路由，避免生产环境 file:// 下的 history 路由问题
    tauri::WebviewWindowBuilder::new(
        app,
        PREFERENCES_WINDOW_LABEL,
        tauri::WebviewUrl::App("/#/preferences".into()),
    )
    .title("偏好设置")
    .inner_size(520.0, 420.0)
    .resizable(true)
    .visible(true)
    .build()?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::menu::MenuEvent;
            use tauri::tray::{TrayIconBuilder, TrayIconEvent};

            let preferences = MenuItem::with_id(app, MENU_PREFERENCES_ID, "偏好设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, MENU_QUIT_ID, "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&preferences, &quit])?;

            let icon = app.default_window_icon().cloned();

            let mut builder = TrayIconBuilder::with_id(TRAY_ID)
                .menu(&menu)
                // 交互约定：右键弹菜单；左键用于“打开主窗口”
                // 不同平台默认行为可能不同，这里显式关闭“左键弹出菜单”。
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event: MenuEvent| {
                match event.id().as_ref() {
                    MENU_PREFERENCES_ID => {
                        let _ = show_preferences_window(app);
                    }
                    MENU_QUIT_ID => {
                        app.exit(0);
                    }
                    _ => {}
                }
            })
                .on_tray_icon_event(|tray, event: TrayIconEvent| {
                    // 交互约定：
                    // - 左键：直接打开主窗口
                    // - 右键：弹出托盘菜单（由系统/tauri 处理）
                    if let TrayIconEvent::Click { button, button_state, .. } = event {
                        if button == tauri::tray::MouseButton::Left
                            && button_state == tauri::tray::MouseButtonState::Up
                        {
                            let _ = show_main_window(tray.app_handle());
                        }
                    }
                });

            if let Some(icon) = icon {
                builder = builder.icon(icon);
            }

            builder.build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // 点击窗口关闭按钮时：隐藏窗口，不退出应用（托盘仍可恢复）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
