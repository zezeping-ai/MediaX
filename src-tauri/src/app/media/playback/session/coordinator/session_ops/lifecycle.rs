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
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, Manager, State};

pub fn open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
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
    {
        let mut playback = state::playback(&state)?;
        playback.open(path.clone());
    }
    state.runtime.stream.set_latest_position_seconds(0.0)?;
    state.runtime.stream.reset_pending_seek_to_zero()?;
    {
        let mut library = state::library(&state)?;
        library.mark_playback_progress(&path, 0.0);
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
    let has_active_stream = state.runtime.stream.has_active_stream().map_err(MediaError::from)?;
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
        emit_debug(&app, "pause_prefetch", "keep network stream alive for paused buffering");
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
    state.runtime.stream.set_latest_position_seconds(0.0)?;
    {
        let mut playback = state::playback(&state)?;
        playback.stop();
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
