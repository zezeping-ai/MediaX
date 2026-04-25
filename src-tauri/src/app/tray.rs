use crate::app::windows::show_main_window;
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

const MENU_TRAY_SHOW_MAIN_ID: &str = "mediax.tray.show_main";

const TRAY_ID: &str = "mediax.tray";

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show_main = MenuItem::with_id(
        app,
        MENU_TRAY_SHOW_MAIN_ID,
        "显示主窗口",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show_main])?;

    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        // 交互约定：右键弹菜单；左键用于「打开主窗口」
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event: MenuEvent| {
            if event.id().as_ref() == MENU_TRAY_SHOW_MAIN_ID {
                let _ = show_main_window(app);
            }
        })
        .on_tray_icon_event(|tray, event: TrayIconEvent| {
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
