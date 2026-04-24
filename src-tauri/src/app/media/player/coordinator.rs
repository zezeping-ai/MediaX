use crate::app::media::player::preview::{generate_preview_frame, render_preview_frame_at};
use crate::app::media::player::renderer::RendererState;
use crate::app::media::player::runtime::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream,
    write_latest_stream_position,
};
use crate::app::media::player::state::MediaState;
use crate::app::media::snapshot::{emit_snapshot_with_request_id, snapshot_from_state};
use crate::app::media::types::{HardwareDecodeMode, MediaSnapshot, PlaybackStatus, PreviewFrame};
use std::sync::atomic::Ordering;
use tauri::{async_runtime, AppHandle, Manager, State};

pub fn get_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    snapshot_from_state(&state)
}

pub fn open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.open(path.clone());
    }
    {
        let mut latest_position = state
            .latest_stream_position_seconds
            .lock()
            .map_err(|_| "latest position state poisoned".to_string())?;
        *latest_position = 0.0;
    }
    {
        let mut pending_seek = state
            .pending_seek_seconds
            .lock()
            .map_err(|_| "pending seek state poisoned".to_string())?;
        *pending_seek = Some(0.0);
    }
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, 0.0);
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn play(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    let latest_stream_position_seconds = read_latest_stream_position(&state).unwrap_or(0.0);
    let (current_path, resume_position_seconds) = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.play();
        let playback_state = playback.state();
        let resume_position_seconds = playback_state
            .position_seconds
            .max(latest_stream_position_seconds)
            .max(0.0);
        playback.seek(resume_position_seconds);
        (playback_state.current_path, resume_position_seconds)
    };
    {
        let mut pending_seek = state
            .pending_seek_seconds
            .lock()
            .map_err(|_| "pending seek state poisoned".to_string())?;
        *pending_seek = Some(resume_position_seconds);
    }
    if let Some(source) = current_path {
        start_decode_stream(&app, &state, source)?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    stop_decode_stream(&state)?;
    let latest_stream_position_seconds = read_latest_stream_position(&state).unwrap_or(0.0);
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        let current_position_seconds = playback.state().position_seconds.max(0.0);
        let resume_position_seconds = current_position_seconds.max(latest_stream_position_seconds);
        playback.seek(resume_position_seconds);
        playback.pause();
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    stop_decode_stream(&state)?;
    {
        let mut latest_position = state
            .latest_stream_position_seconds
            .lock()
            .map_err(|_| "latest position state poisoned".to_string())?;
        *latest_position = 0.0;
    }
    let current_path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        let path = playback.state().current_path;
        playback.stop();
        path
    };
    {
        let mut pending_seek = state
            .pending_seek_seconds
            .lock()
            .map_err(|_| "pending seek state poisoned".to_string())?;
        *pending_seek = Some(0.0);
    }
    if let Some(source) = current_path {
        let epoch = state.paused_seek_epoch.fetch_add(1, Ordering::Relaxed) + 1;
        let app_handle = app.clone();
        let renderer = (*app_handle.state::<RendererState>()).clone();
        async_runtime::spawn_blocking(move || {
            if let Err(err) = render_paused_seek_frame(&app_handle, &renderer, &source, 0.0, epoch) {
                eprintln!("stop preview failed: {err}");
            }
        });
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    force_render: Option<bool>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    let (path, status) = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.seek(position_seconds);
        let playback_state = playback.state();
        (playback_state.current_path, playback_state.status)
    };
    if let Some(path) = path.as_deref() {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    {
        let mut pending_seek = state
            .pending_seek_seconds
            .lock()
            .map_err(|_| "pending seek state poisoned".to_string())?;
        *pending_seek = Some(position_seconds.max(0.0));
    }
    if status == PlaybackStatus::Paused {
        let should_force_render = force_render.unwrap_or(false);
        if let Some(source) = path {
            let target = position_seconds.max(0.0);
            let epoch = state.paused_seek_epoch.fetch_add(1, Ordering::Relaxed) + 1;
            let renderer = (*app.state::<RendererState>()).clone();
            if should_force_render {
                let app_handle = app.clone();
                if let Err(err) = render_paused_seek_frame(&app_handle, &renderer, &source, target, epoch) {
                    eprintln!("preview seek force render failed: {err}");
                }
            } else {
                let app_handle = app.clone();
                async_runtime::spawn_blocking(move || {
                    if let Err(err) = render_paused_seek_frame(&app_handle, &renderer, &source, target, epoch) {
                        eprintln!("preview seek failed: {err}");
                    }
                });
            }
        }
    }
    write_latest_stream_position(&state, position_seconds.max(0.0))?;
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.set_rate(playback_rate);
    }
    state.timing_controls.set_playback_rate(playback_rate as f32);
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    state.audio_controls.set_volume(volume as f32);
    if volume <= 0.0 {
        state.audio_controls.set_muted(true);
    } else {
        state.audio_controls.set_muted(false);
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    state.audio_controls.set_muted(muted);
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn set_hw_decode_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.set_hw_decode_mode(mode);
        playback.update_hw_decode_status(false, None, None);
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub fn sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    let path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.sync_position(position_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot_with_request_id(&app, &state, request_id)
}

pub async fn preview_frame(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Result<Option<PreviewFrame>, String> {
    let source = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.state().current_path
    };
    let Some(source) = source else {
        return Ok(None);
    };

    let epoch = state.preview_frame_epoch.fetch_add(1, Ordering::Relaxed) + 1;
    let width = max_width.unwrap_or(160);
    let height = max_height.unwrap_or(90);
    let target = position_seconds.max(0.0);
    let app_handle = app.clone();
    async_runtime::spawn_blocking(move || {
        generate_preview_frame(&source, target, width, height, || {
            app_handle
                .state::<MediaState>()
                .preview_frame_epoch
                .load(Ordering::Relaxed)
                != epoch
        })
    })
    .await
    .map_err(|err| format!("preview task join failed: {err}"))?
}

fn render_paused_seek_frame(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    target: f64,
    epoch: u32,
) -> Result<(), String> {
    render_preview_frame_at(renderer, source, target, || {
        app.state::<MediaState>()
            .paused_seek_epoch
            .load(Ordering::Relaxed)
            != epoch
    })
}
