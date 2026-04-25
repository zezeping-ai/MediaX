mod app;
use app::media::player::renderer::RendererState;
use app::media::player::state::MediaState;
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
            app::media::player::commands::playback_get_snapshot,
            app::media::library_commands::media_set_library_roots,
            app::media::library_commands::media_rescan_library,
            app::media::player::commands::playback_open_source,
            app::media::player::commands::playback_resume,
            app::media::player::commands::playback_pause,
            app::media::player::commands::playback_stop_session,
            app::media::player::commands::playback_seek_to,
            app::media::player::commands::playback_set_rate,
            app::media::player::commands::playback_set_volume,
            app::media::player::commands::playback_set_muted,
            app::media::player::commands::playback_configure_decoder_mode,
            app::media::player::commands::playback_set_quality,
            app::media::player::commands::playback_sync_position,
            app::media::player::commands::playback_preview_frame,
            app::media::player::commands::playback_get_cache_recording_status,
            app::media::player::commands::playback_start_cache_recording,
            app::media::player::commands::playback_stop_cache_recording,
            app::windows::window_set_main_always_on_top,
            app::windows::window_set_main_video_scale_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
