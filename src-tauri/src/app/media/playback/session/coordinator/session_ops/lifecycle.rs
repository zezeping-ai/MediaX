use super::super::helpers::{
    activate_playback_and_resume_position, finalize_active_cache_recording, set_pending_seek,
    sync_pause_resume_position,
};
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::runtime::{start_decode_stream, stop_decode_stream_non_blocking};
use crate::app::media::playback::session::service::supports_timeline_seek;
use crate::app::media::playback::session::source_path::normalize_playable_source;
use crate::app::media::playback::session::player_settings;
use crate::app::media::state;
use crate::app::media::state::emit_snapshot_with_request_id;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager, State};

pub fn open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
    resume_last_position: Option<bool>,
) -> MediaResult<MediaSnapshot> {
    let path = normalize_playable_source(path)?;
    finalize_active_cache_recording(&state, "播放源已切换，录制已自动停止")?;
    state
        .runtime
        .pause_prefetch_active
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state.runtime.stream.advance_generation();
    stop_decode_stream_non_blocking(&state)?;
    if let Err(err) = (*app.state::<RendererState>()).clone().clear_surface(&app) {
        eprintln!("clear renderer surface on source switch failed: {err}");
    }
    let resume_enabled = resume_last_position
        .unwrap_or_else(|| player_settings::resume_last_position_enabled(&state));
    let resume_position_seconds = if resume_enabled {
        let library = state::library(&state)?;
        library.saved_position_seconds(&path)
    } else {
        0.0
    };
    {
        let mut playback = state::playback(&state)?;
        playback.open(path.clone());
        if resume_position_seconds > f64::EPSILON {
            playback.seek(resume_position_seconds);
        }
    }
    state
        .runtime
        .stream
        .set_latest_position_seconds(resume_position_seconds)?;
    if resume_position_seconds > f64::EPSILON {
        set_pending_seek(&state, resume_position_seconds)?;
    } else {
        state.runtime.stream.reset_pending_seek_to_zero()?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn play(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let (current_path, resume_position_seconds) = activate_playback_and_resume_position(&state)?;
    let resume_prefetch_stream = state
        .runtime
        .pause_prefetch_active
        .swap(false, std::sync::atomic::Ordering::Relaxed);
    let has_active_stream = state
        .runtime
        .stream
        .has_active_stream()?;
    let should_restart_stream = !resume_prefetch_stream || !has_active_stream;
    if let Some(source) = current_path.as_deref() {
        if should_restart_stream && supports_timeline_seek(source) {
            set_pending_seek(&state, resume_position_seconds)?;
        } else if resume_position_seconds > f64::EPSILON {
            emit_debug(
                &app,
                "seek_ignored",
                format!(
                    "skip resume seek for seek-limited source target={resume_position_seconds:.3}s source={source}"
                ),
            );
        }
    }
    if let Some(source) = current_path {
        if should_restart_stream {
            start_decode_stream(&app, &state, source)?;
        } else {
            emit_debug(
                &app,
                "pause_prefetch_resume",
                format!("resume buffered stream at {resume_position_seconds:.3}s"),
            );
        }
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let should_keep_prefetch_alive = {
        let playback = state::playback(&state)?;
        playback
            .state()
            .current_path
            .as_deref()
            .is_some_and(should_use_pause_prefetch)
    };
    if should_keep_prefetch_alive {
        state
            .runtime
            .pause_prefetch_active
            .store(true, std::sync::atomic::Ordering::Relaxed);
        emit_debug(
            &app,
            "pause_prefetch",
            "keep network stream alive for paused buffering",
        );
    } else {
        state
            .runtime
            .pause_prefetch_active
            .store(false, std::sync::atomic::Ordering::Relaxed);
        state.runtime.stream.advance_generation();
        stop_decode_stream_non_blocking(&state)?;
    }
    sync_pause_resume_position(&state)?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    finalize_active_cache_recording(&state, "播放已停止，录制已自动停止")?;
    state
        .runtime
        .pause_prefetch_active
        .store(false, std::sync::atomic::Ordering::Relaxed);
    state.runtime.stream.advance_generation();
    stop_decode_stream_non_blocking(&state)?;
    let stop_progress = {
        let playback = state::playback(&state)?;
        let playback_state = playback.state();
        (
            playback_state.current_path.clone(),
            playback_state.position_seconds.max(0.0),
            playback_state.duration_seconds,
        )
    };
    state.runtime.stream.set_latest_position_seconds(0.0)?;
    {
        let mut playback = state::playback(&state)?;
        playback.stop();
    }
    if let Some(path) = stop_progress.0.as_deref() {
        let mut library = state::library(&state)?;
        let duration = (stop_progress.2 > 0.0).then_some(stop_progress.2);
        library.mark_playback_progress(path, stop_progress.1, duration);
    }
    state.runtime.stream.reset_pending_seek_to_zero()?;
    if let Err(err) = (*app.state::<RendererState>()).clone().clear_surface(&app) {
        eprintln!("clear renderer surface on stop failed: {err}");
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

fn should_use_pause_prefetch(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    supports_timeline_seek(source)
        && (normalized.starts_with("http://")
            || normalized.starts_with("https://")
            || normalized.starts_with("rtsp://")
            || normalized.starts_with("rtmp://")
            || normalized.starts_with("mms://"))
}
