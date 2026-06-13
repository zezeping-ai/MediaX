use crate::app::media::playback::dto::{PlaybackState, PlaybackStatus};
use crate::app::media::playback::session::coordinator;
use crate::app::media::state::MediaState;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[cfg(target_os = "windows")]
const MAIN_WINDOW_LABEL: &str = "main";
const SKIP_SECONDS: f64 = 15.0;
const RESTART_THRESHOLD_SECONDS: f64 = 3.0;

pub struct SystemMediaControlsState {
    controls: Mutex<Option<MediaControls>>,
    metadata_cache: Mutex<CachedMetadata>,
}

#[derive(Default)]
struct CachedMetadata {
    title: String,
    artist: String,
    album: String,
}

pub fn setup(app: &AppHandle) -> Result<(), String> {
    let config = PlatformConfig {
        dbus_name: "com.zezeping.mediax",
        display_name: "MediaX",
        hwnd: resolve_main_window_hwnd(app),
    };

    let mut controls = MediaControls::new(config).map_err(|err| {
        format!("initialize system media controls failed: {err}")
    })?;

    let app_handle = app.clone();
    controls
        .attach(move |event| dispatch_media_event(app_handle.clone(), event))
        .map_err(|err| format!("attach system media controls handler failed: {err}"))?;

    app.manage(SystemMediaControlsState {
        controls: Mutex::new(Some(controls)),
        metadata_cache: Mutex::new(CachedMetadata::default()),
    });
    Ok(())
}

pub fn sync_playback_state(app: &AppHandle, playback: &PlaybackState) {
    let Some(state) = app.try_state::<SystemMediaControlsState>() else {
        return;
    };

    let mut metadata_cache = match state.metadata_cache.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    update_metadata_cache(&mut metadata_cache, playback);

    let mut controls_guard = match state.controls.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let Some(controls) = controls_guard.as_mut() else {
        return;
    };

    let metadata = build_metadata(&metadata_cache, playback);
    if let Err(err) = controls.set_metadata(metadata) {
        eprintln!("system media controls set metadata failed: {err}");
    }

    let playback_state = build_playback_state(playback);
    if let Err(err) = controls.set_playback(playback_state) {
        eprintln!("system media controls set playback failed: {err}");
    }
}

fn resolve_main_window_hwnd(app: &AppHandle) -> Option<*mut std::ffi::c_void> {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let window = app.get_webview_window(MAIN_WINDOW_LABEL)?;
        let handle = window.window_handle().ok()?;
        return match handle.as_raw() {
            RawWindowHandle::Win32(win) => Some(win.hwnd.get() as *mut std::ffi::c_void),
            _ => None,
        };
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        None
    }
}

fn dispatch_media_event(app: AppHandle, event: MediaControlEvent) {
    let _ = app.clone().run_on_main_thread(move || {
        if let Err(err) = handle_media_control_event(&app, event) {
            eprintln!("system media control event failed: {err}");
        }
    });
}

fn handle_media_control_event(app: &AppHandle, event: MediaControlEvent) -> Result<(), String> {
    let state = app.state::<MediaState>();
    match event {
        MediaControlEvent::Play => {
            coordinator::play(app.clone(), state, None).map_err(|err| err.to_string())?;
        }
        MediaControlEvent::Pause => {
            coordinator::pause(app.clone(), state, None).map_err(|err| err.to_string())?;
        }
        MediaControlEvent::Toggle => {
            let status = state
                .session
                .playback
                .lock()
                .map_err(|_| "playback state poisoned".to_string())?
                .state()
                .status;
            match status {
                PlaybackStatus::Playing => {
                    coordinator::pause(app.clone(), state, None).map_err(|err| err.to_string())?;
                }
                _ => {
                    coordinator::play(app.clone(), state, None).map_err(|err| err.to_string())?;
                }
            }
        }
        MediaControlEvent::Stop => {
            coordinator::stop(app.clone(), state, None).map_err(|err| err.to_string())?;
        }
        MediaControlEvent::Next => {
            seek_relative(app, state, SKIP_SECONDS)?;
        }
        MediaControlEvent::Previous => {
            let position = state
                .session
                .playback
                .lock()
                .map_err(|_| "playback state poisoned".to_string())?
                .state()
                .position_seconds;
            let delta = if position > RESTART_THRESHOLD_SECONDS {
                -SKIP_SECONDS
            } else {
                -position
            };
            seek_relative(app, state, delta)?;
        }
        MediaControlEvent::Seek(direction) => {
            let delta = match direction {
                SeekDirection::Forward => SKIP_SECONDS,
                SeekDirection::Backward => -SKIP_SECONDS,
            };
            seek_relative(app, state, delta)?;
        }
        MediaControlEvent::SeekBy(direction, duration) => {
            let seconds = duration.as_secs_f64();
            let delta = match direction {
                SeekDirection::Forward => seconds,
                SeekDirection::Backward => -seconds,
            };
            seek_relative(app, state, delta)?;
        }
        MediaControlEvent::SetPosition(position) => {
            coordinator::seek(
                app.clone(),
                state,
                position.0.as_secs_f64(),
                None,
                None,
            )
            .map_err(|err| err.to_string())?;
        }
        MediaControlEvent::Raise => {
            let _ = crate::app::windows::show_main_window(app);
        }
        MediaControlEvent::SetVolume(volume) => {
            coordinator::set_volume(app.clone(), state, volume, None)
                .map_err(|err| err.to_string())?;
        }
        MediaControlEvent::OpenUri(_)
        | MediaControlEvent::Quit => {}
    }
    Ok(())
}

fn seek_relative(app: &AppHandle, state: tauri::State<'_, MediaState>, delta: f64) -> Result<(), String> {
    let (position, duration) = {
        let playback = state
            .session
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        let snapshot = playback.state();
        (snapshot.position_seconds, snapshot.duration_seconds)
    };
    let max = if duration > 0.0 { duration } else { f64::MAX };
    let target = (position + delta).clamp(0.0, max);
    coordinator::seek(app.clone(), state, target, None, None).map_err(|err| err.to_string())?;
    Ok(())
}

fn update_metadata_cache(cache: &mut CachedMetadata, playback: &PlaybackState) {
    cache.title = display_title(playback);
    cache.artist = playback
        .artist
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    cache.album = playback
        .album
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
}

fn build_metadata<'a>(
    cache: &'a CachedMetadata,
    playback: &PlaybackState,
) -> MediaMetadata<'a> {
    MediaMetadata {
        title: Some(cache.title.as_str()),
        artist: optional_str(&cache.artist),
        album: optional_str(&cache.album),
        cover_url: None,
        duration: positive_duration(playback.duration_seconds),
    }
}

fn build_playback_state(playback: &PlaybackState) -> MediaPlayback {
    let progress = positive_duration(playback.position_seconds).map(MediaPosition);
    match playback.status {
        PlaybackStatus::Playing => MediaPlayback::Playing { progress },
        PlaybackStatus::Paused => MediaPlayback::Paused { progress },
        PlaybackStatus::Idle | PlaybackStatus::Stopped => MediaPlayback::Stopped,
    }
}

fn display_title(playback: &PlaybackState) -> String {
    if let Some(title) = playback.title.as_deref().filter(|value| !value.is_empty()) {
        return title.to_string();
    }
    playback
        .current_path
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("MediaX")
        .to_string()
}

fn optional_str(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn positive_duration(seconds: f64) -> Option<Duration> {
    if seconds.is_finite() && seconds > 0.0 {
        Some(Duration::from_secs_f64(seconds))
    } else {
        None
    }
}
