mod app;
use app::commands::MediaState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MediaState::default())
        .setup(|app| {
            app::menu::setup(app)?;
            app::tray::setup(app)?;
            Ok(())
        })
        .on_menu_event(app::menu::handle_menu_event)
        .on_window_event(app::windows::handle_close_requested)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            app::commands::media_get_snapshot,
            app::commands::media_set_library_roots,
            app::commands::media_rescan_library,
            app::commands::media_open,
            app::commands::media_play,
            app::commands::media_pause,
            app::commands::media_stop,
            app::commands::media_seek,
            app::commands::media_set_rate,
            app::commands::media_sync_position
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
