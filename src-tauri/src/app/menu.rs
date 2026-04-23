use tauri::menu::{Menu, MenuEvent, MenuItem, Submenu};
use tauri::Emitter;

pub const APP_MENU_EVENT: &str = "media://menu-action";

const MENU_FILE_OPEN_LOCAL_ID: &str = "mediax.file.open_local";
const MENU_FILE_OPEN_URL_ID: &str = "mediax.file.open_url";

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let open_local = MenuItem::with_id(app, MENU_FILE_OPEN_LOCAL_ID, "打开本地文件...", true, None::<&str>)?;
    let open_url = MenuItem::with_id(app, MENU_FILE_OPEN_URL_ID, "打开 URL...", true, None::<&str>)?;

    let file_submenu = Submenu::with_items(app, "File", true, &[&open_local, &open_url])?;
    let menu = Menu::with_items(app, &[&file_submenu])?;
    app.set_menu(menu)?;
    Ok(())
}

pub fn handle_menu_event(app: &tauri::AppHandle, event: MenuEvent) {
    let action = match event.id().as_ref() {
        MENU_FILE_OPEN_LOCAL_ID => Some("open_local"),
        MENU_FILE_OPEN_URL_ID => Some("open_url"),
        _ => None,
    };
    if let Some(action) = action {
        let _ = app.emit(APP_MENU_EVENT, action);
    }
}
