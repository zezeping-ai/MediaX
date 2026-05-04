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
#[cfg(target_os = "macos")]
use tauri::RunEvent;
#[cfg(desktop)]
use tauri_plugin_single_instance;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            app::launch::handle_secondary_launch(app, &args);
        }));
    }

    let app = builder
        .manage(MediaState::default())
        .manage(RendererState::new())
        .setup(|app| {
            initialize_playback_debug_log(app.handle()).map_err(|err| {
                let boxed: Box<dyn std::error::Error> = Box::new(std::io::Error::other(err));
                tauri::Error::Setup(boxed.into())
            })?;
            app::shell::menu::setup(app)?;
            app::shell::tray::setup(app)?;
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
            app::launch::bootstrap_from_launch_sources(app.handle());
            #[cfg(desktop)]
            app::launch::bootstrap_from_deep_links(app.handle()).map_err(|err| {
                let boxed: Box<dyn std::error::Error> = Box::new(std::io::Error::other(err));
                tauri::Error::Setup(boxed.into())
            })?;
            Ok(())
        })
        .on_menu_event(app::shell::menu::handle_menu_event)
        .on_window_event(app::windows::handle_close_requested)
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            playback_session_commands::playback_get_snapshot,
            library::media_set_library_roots,
            library::media_rescan_library,
            playback_session_commands::playback_open_source,
            playback_session_commands::playback_pick_local_file,
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
            app::windows::commands::window_set_main_always_on_top,
            app::windows::commands::window_set_main_video_scale_mode,
            app::windows::commands::window_toggle_main_fullscreen,
            app::windows::commands::window_start_main_dragging,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let RunEvent::Opened { urls } = event {
            app::launch::handle_opened_urls(app_handle, &urls);
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = app_handle;
            let _ = event;
        }
    });
}
