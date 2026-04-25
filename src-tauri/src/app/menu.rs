use crate::app::updates::check_and_install_update;
use crate::app::windows::show_preferences_window;
use tauri::menu::{
    AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID,
};
use tauri::Emitter;

pub const APP_MENU_EVENT: &str = "media://menu-action";

const MENU_APP_SETTINGS_ID: &str = "mediax.app.settings";
const MENU_APP_QUIT_ID: &str = "mediax.app.quit";
const MENU_FILE_OPEN_LOCAL_ID: &str = "mediax.file.open_local";
const MENU_FILE_OPEN_URL_ID: &str = "mediax.file.open_url";
const MENU_HELP_CHECK_UPDATE_ID: &str = "mediax.help.check_update";

fn app_menu_title(app: &tauri::App) -> String {
    let name = app.package_info().name.clone();
    let mut chars = name.chars();
    match chars.next() {
        None => name,
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let pkg = app.package_info();
    let about_metadata = AboutMetadata {
        name: Some(pkg.name.clone()),
        version: Some(pkg.version.to_string()),
        copyright: app.config().bundle.copyright.clone(),
        authors: app.config().bundle.publisher.clone().map(|p| vec![p]),
        ..Default::default()
    };
    let about_text = format!("关于 {}", app_menu_title(app));
    let about = PredefinedMenuItem::about(app, Some(about_text.as_str()), Some(about_metadata))?;
    let sep_after_about = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(
        app,
        MENU_APP_SETTINGS_ID,
        "系统设置",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let quit = MenuItem::with_id(app, MENU_APP_QUIT_ID, "退出", true, Some("CmdOrCtrl+Q"))?;

    let app_submenu = Submenu::new(app, app_menu_title(app), true)?;
    app_submenu.append(&about)?;
    app_submenu.append(&sep_after_about)?;
    app_submenu.append(&settings)?;

    #[cfg(target_os = "macos")]
    {
        let sep = PredefinedMenuItem::separator(app)?;
        let services = PredefinedMenuItem::services(app, None)?;
        let sep2 = PredefinedMenuItem::separator(app)?;
        let hide = PredefinedMenuItem::hide(app, None)?;
        let hide_others = PredefinedMenuItem::hide_others(app, None)?;
        let sep3 = PredefinedMenuItem::separator(app)?;
        let show_all = PredefinedMenuItem::show_all(app, None)?;
        let sep_before_quit = PredefinedMenuItem::separator(app)?;
        app_submenu.append(&sep)?;
        app_submenu.append(&services)?;
        app_submenu.append(&sep2)?;
        app_submenu.append(&hide)?;
        app_submenu.append(&hide_others)?;
        app_submenu.append(&sep3)?;
        app_submenu.append(&show_all)?;
        app_submenu.append(&sep_before_quit)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        let sep_before_quit = PredefinedMenuItem::separator(app)?;
        app_submenu.append(&sep_before_quit)?;
    }

    app_submenu.append(&quit)?;

    let open_local = MenuItem::with_id(
        app,
        MENU_FILE_OPEN_LOCAL_ID,
        "打开本地文件...",
        true,
        None::<&str>,
    )?;
    let open_url = MenuItem::with_id(
        app,
        MENU_FILE_OPEN_URL_ID,
        "打开 URL...",
        true,
        None::<&str>,
    )?;
    let file_submenu = Submenu::with_items(app, "File", true, &[&open_local, &open_url])?;

    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;
    let edit_submenu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[&undo, &redo, &cut, &copy, &paste, &select_all],
    )?;

    let check_update = MenuItem::with_id(
        app,
        MENU_HELP_CHECK_UPDATE_ID,
        "检查更新",
        true,
        None::<&str>,
    )?;
    let help_submenu =
        Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Help", true, &[&check_update])?;

    let menu = Menu::with_items(
        app,
        &[&app_submenu, &file_submenu, &edit_submenu, &help_submenu],
    )?;
    app.set_menu(menu)?;
    Ok(())
}

pub fn handle_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        MENU_APP_QUIT_ID => {
            app.exit(0);
        }
        MENU_APP_SETTINGS_ID => {
            let _ = show_preferences_window(app);
        }
        MENU_HELP_CHECK_UPDATE_ID => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                check_and_install_update(app).await;
            });
        }
        MENU_FILE_OPEN_LOCAL_ID => {
            let _ = app.emit(APP_MENU_EVENT, "open_local");
        }
        MENU_FILE_OPEN_URL_ID => {
            let _ = app.emit(APP_MENU_EVENT, "open_url");
        }
        _ => {}
    }
}
