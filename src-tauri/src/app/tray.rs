use crate::app::updates::check_and_install_update;
use crate::app::windows::{show_main_window, show_preferences_window};
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

const TRAY_ID: &str = "mediax.tray";
const MENU_PREFERENCES_ID: &str = "mediax.menu.preferences";
const MENU_CHECK_UPDATE_ID: &str = "mediax.menu.check_update";
const MENU_QUIT_ID: &str = "mediax.menu.quit";

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let preferences = MenuItem::with_id(app, MENU_PREFERENCES_ID, "偏好设置", true, None::<&str>)?;
    let check_update =
        MenuItem::with_id(app, MENU_CHECK_UPDATE_ID, "检查更新", true, None::<&str>)?;
    let separator_top = PredefinedMenuItem::separator(app)?;
    let separator_bottom = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, MENU_QUIT_ID, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &preferences,
            &separator_top,
            &check_update,
            &separator_bottom,
            &quit,
        ],
    )?;

    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        // 交互约定：右键弹菜单；左键用于“打开主窗口”
        // 不同平台默认行为可能不同，这里显式关闭“左键弹出菜单”。
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event: MenuEvent| match event.id().as_ref() {
            MENU_PREFERENCES_ID => {
                let _ = show_preferences_window(app);
            }
            MENU_CHECK_UPDATE_ID => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    check_and_install_update(app).await;
                });
            }
            MENU_QUIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event: TrayIconEvent| {
            // 交互约定：
            // - 左键：直接打开主窗口
            // - 右键：弹出托盘菜单（由系统/tauri 处理）
            if let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            {
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
}
