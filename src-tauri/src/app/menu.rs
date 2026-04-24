use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::Emitter;

pub const APP_MENU_EVENT: &str = "media://menu-action";

const MENU_FILE_OPEN_LOCAL_ID: &str = "mediax.file.open_local";
const MENU_FILE_OPEN_URL_ID: &str = "mediax.file.open_url";
const MENU_FILE_QUIT_ID: &str = "mediax.file.quit";

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let open_local = MenuItem::with_id(app, MENU_FILE_OPEN_LOCAL_ID, "打开本地文件...", true, None::<&str>)?;
    let open_url = MenuItem::with_id(app, MENU_FILE_OPEN_URL_ID, "打开 URL...", true, None::<&str>)?;
    let separator = MenuItem::with_id(app, "mediax.file.separator", "-", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_FILE_QUIT_ID, "退出", true, Some("CmdOrCtrl+Q"))?;

    // Use predefined menu items so the platform/webview gets native edit roles.
    // This is required for Cmd/Ctrl+C/V/X/A/Z shortcuts to work reliably.
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;

    let file_submenu =
        Submenu::with_items(app, "File", true, &[&open_local, &open_url, &separator, &quit])?;
    let edit_submenu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[&undo, &redo, &cut, &copy, &paste, &select_all],
    )?;
    let menu = Menu::with_items(app, &[&file_submenu, &edit_submenu])?;
    app.set_menu(menu)?;
    Ok(())
}

pub fn handle_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    if event.id().as_ref() == MENU_FILE_QUIT_ID {
        app.exit(0);
        return;
    }

    let action = match event.id().as_ref() {
        MENU_FILE_OPEN_LOCAL_ID => Some("open_local"),
        MENU_FILE_OPEN_URL_ID => Some("open_url"),
        _ => None,
    };
    if let Some(action) = action {
        let _ = app.emit(APP_MENU_EVENT, action);
    }
}
