mod restart;

use super::helpers::{
    activate_playback_and_resume_position, finalize_active_cache_recording, set_pending_seek,
    sync_pause_resume_position,
};
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::{
    HardwareDecodeMode, MediaSnapshot, PlaybackMediaKind, PlaybackQualityMode, PlaybackStatus,
};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::render::viewport_sync;
use crate::app::media::playback::runtime::{
    start_decode_stream, stop_decode_stream_non_blocking, write_latest_stream_position,
};
use crate::app::media::playback::session::constraints;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, Manager, State};

use self::restart::restart_active_playback;

pub fn open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    finalize_active_cache_recording(&state, "播放源已切换，录制已自动停止")?;
    state.stream.advance_generation();
    stop_decode_stream_non_blocking(&state)?;
    if let Err(err) = (*app.state::<RendererState>()).clone().clear_surface(&app) {
        eprintln!("clear renderer surface on source switch failed: {err}");
    }
    {
        let mut playback = state::playback(&state)?;
        playback.open(path.clone());
    }
    state.stream.set_latest_position_seconds(0.0)?;
    state.stream.reset_pending_seek_to_zero()?;
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
    set_pending_seek(&state, resume_position_seconds)?;
    if let Some(source) = current_path {
        start_decode_stream(&app, &state, source)?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn pause(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    state.stream.advance_generation();
    stop_decode_stream_non_blocking(&state)?;
    sync_pause_resume_position(&state)?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    finalize_active_cache_recording(&state, "播放已停止，录制已自动停止")?;
    state.stream.advance_generation();
    stop_decode_stream_non_blocking(&state)?;
    state.stream.set_latest_position_seconds(0.0)?;
    {
        let mut playback = state::playback(&state)?;
        playback.stop();
    }
    state.stream.reset_pending_seek_to_zero()?;
    if let Err(err) = (*app.state::<RendererState>()).clone().clear_surface(&app) {
        eprintln!("clear renderer surface on stop failed: {err}");
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    _force_render: Option<bool>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let position_seconds =
        constraints::normalize_non_negative(position_seconds, "position_seconds")?;
    let (media_path, status, media_kind) = {
        let mut playback = state::playback(&state)?;
        playback.seek(position_seconds);
        let playback_state = playback.state();
        (
            playback_state.current_path,
            playback_state.status,
            playback_state.media_kind,
        )
    };
    if let Some(path_ref) = media_path.as_deref() {
        let mut library = state::library(&state)?;
        library.mark_playback_progress(path_ref, position_seconds);
    }
    set_pending_seek(&state, position_seconds)?;
    write_latest_stream_position(&state, position_seconds.max(0.0))?;
    apply_paused_seek_preview(&app, &state, media_path.as_deref(), &status, &media_kind, position_seconds);
    if should_restart_playback_after_seek(&status, &media_kind) {
        restart_active_playback(&app, &state, media_path)?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

fn apply_paused_seek_preview(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: Option<&str>,
    status: &PlaybackStatus,
    media_kind: &PlaybackMediaKind,
    position_seconds: f64,
) {
    if *status != PlaybackStatus::Paused || *media_kind != PlaybackMediaKind::Video {
        return;
    }
    let Some(source) = source else {
        return;
    };
    let target = position_seconds.max(0.0);
    let renderer = (*app.state::<RendererState>()).clone();
    if let Err(err) = viewport_sync::sync_main_viewport_to(state, &renderer, source, target) {
        eprintln!("paused seek preview failed: {err}");
    }
}

fn should_restart_playback_after_seek(
    status: &PlaybackStatus,
    media_kind: &PlaybackMediaKind,
) -> bool {
    *status == PlaybackStatus::Playing && *media_kind == PlaybackMediaKind::Audio
}

pub fn set_hw_decode_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playing_path = {
        let mut playback = state::playback(&state)?;
        if playback.hw_decode_mode() == mode {
            return emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from);
        }
        playback.set_hw_decode_mode(mode);
        playback.update_hw_decode_status(false, None, None);
        let st = playback.state();
        if st.status == PlaybackStatus::Playing {
            st.current_path.clone()
        } else {
            None
        }
    };

    restart_active_playback(&app, &state, playing_path)?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_quality_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: PlaybackQualityMode,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playing_path = {
        let mut playback = state::playback(&state)?;
        if mode == PlaybackQualityMode::Auto && !playback.adaptive_quality_supported() {
            return Err(MediaError::invalid_input(
                "adaptive quality is not supported for current source",
            ));
        }
        playback.set_quality_mode(mode);
        let st = playback.state();
        if st.status == PlaybackStatus::Playing {
            st.current_path.clone()
        } else {
            None
        }
    };

    restart_active_playback(&app, &state, playing_path)?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}
