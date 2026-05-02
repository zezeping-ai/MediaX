mod app;
use app::media::{
    library,
    playback::debug_log::initialize_playback_debug_log,
    playback::session::commands::{
        cache as playback_cache_commands, preview as playback_preview_commands,
        session as playback_session_commands, timing as playback_timing_commands,
    },
    MediaState, RendererState,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MediaState::default())
        .manage(RendererState::new())
        .setup(|app| {
            initialize_playback_debug_log(app.handle()).map_err(|err| {
                let boxed: Box<dyn std::error::Error> = Box::new(std::io::Error::other(err));
                tauri::Error::Setup(boxed.into())
            })?;
            app::menu::setup(app)?;
            app::tray::setup(app)?;
            // Milestone 0: start wgpu underlay test rendering.
            let renderer = app.state::<RendererState>();
            renderer.start_render_loop(app.handle()).map_err(|err| {
                let boxed: Box<dyn std::error::Error> = Box::new(std::io::Error::other(err));
                tauri::Error::Setup(boxed.into())
            })?;
            app::autoprobe::bootstrap_from_env(app.handle()).map_err(|err| {
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
            playback_session_commands::playback_get_snapshot,
            library::media_set_library_roots,
            library::media_rescan_library,
            playback_session_commands::playback_open_source,
            playback_session_commands::playback_resume,
            playback_session_commands::playback_pause,
            playback_session_commands::playback_stop_session,
            playback_timing_commands::playback_seek_to,
            playback_timing_commands::playback_set_rate,
            playback_timing_commands::playback_set_volume,
            playback_timing_commands::playback_set_muted,
            playback_timing_commands::playback_set_left_channel_volume,
            playback_timing_commands::playback_set_right_channel_volume,
            playback_timing_commands::playback_set_left_channel_muted,
            playback_timing_commands::playback_set_right_channel_muted,
            playback_timing_commands::playback_set_channel_routing,
            playback_timing_commands::playback_configure_decoder_mode,
            playback_timing_commands::playback_set_quality,
            playback_timing_commands::playback_sync_position,
            playback_preview_commands::playback_preview_frame,
            playback_cache_commands::playback_get_cache_recording_status,
            playback_cache_commands::playback_start_cache_recording,
            playback_cache_commands::playback_stop_cache_recording,
            app::windows::window_set_main_always_on_top,
            app::windows::window_set_main_video_scale_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
