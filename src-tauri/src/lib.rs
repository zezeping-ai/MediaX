mod app;
use app::media::{
    library, playback::session::commands as playback_commands, MediaState, RendererState,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MediaState::default())
        .manage(RendererState::new())
        .setup(|app| {
            app::menu::setup(app)?;
            app::tray::setup(app)?;
            // Milestone 0: start wgpu underlay test rendering.
            let renderer = app.state::<RendererState>();
            renderer.start_render_loop(app.handle()).map_err(|err| {
                let boxed: Box<dyn std::error::Error> = Box::new(std::io::Error::other(err));
                tauri::Error::Setup(boxed.into())
            })?;
            Ok(())
        })
        .on_menu_event(app::menu::handle_menu_event)
        .on_window_event(app::windows::handle_close_requested)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            playback_commands::playback_get_snapshot,
            library::media_set_library_roots,
            library::media_rescan_library,
            playback_commands::playback_open_source,
            playback_commands::playback_resume,
            playback_commands::playback_pause,
            playback_commands::playback_stop_session,
            playback_commands::playback_seek_to,
            playback_commands::playback_set_rate,
            playback_commands::playback_set_volume,
            playback_commands::playback_set_muted,
            playback_commands::playback_configure_decoder_mode,
            playback_commands::playback_set_quality,
            playback_commands::playback_sync_position,
            playback_commands::playback_preview_frame,
            playback_commands::playback_get_cache_recording_status,
            playback_commands::playback_start_cache_recording,
            playback_commands::playback_stop_cache_recording,
            app::windows::window_set_main_always_on_top,
            app::windows::window_set_main_video_scale_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
