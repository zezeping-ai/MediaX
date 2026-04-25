//! Tauri command orchestration: acquire [`MediaState`] locks, drive decode/runtime, sync viewport, emit snapshots.
//!
//! Heavy policy lives in [`crate::app::media::player::viewport_sync`] and [`crate::app::media::player::runtime`].

use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::player::preview::generate_preview_frame;
use crate::app::media::player::renderer::RendererState;
use crate::app::media::player::runtime::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
    stop_decode_stream_non_blocking,
    write_latest_stream_position,
};
use crate::app::media::player::state::MediaState;
use crate::app::media::player::state_locks;
use crate::app::media::player::viewport_sync;
use crate::app::media::snapshot::{emit_snapshot_with_request_id, snapshot_from_state};
use crate::app::media::types::{
    CacheRecordingStatus, HardwareDecodeMode, MediaSnapshot, PlaybackQualityMode, PlaybackStatus,
    PreviewFrame,
};
use crate::app::media::player::state::CacheRecorderSession;
use std::fs;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{async_runtime, AppHandle, Manager, State};

const MIN_PLAYBACK_RATE: f64 = 0.25;
const MAX_PLAYBACK_RATE: f64 = 4.0;
const MIN_PREVIEW_EDGE: u32 = 32;
const MAX_PREVIEW_EDGE: u32 = 4096;

pub fn get_snapshot(state: State<'_, MediaState>) -> MediaResult<MediaSnapshot> {
    snapshot_from_state(&state).map_err(MediaError::from)
}

pub fn open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    finalize_active_cache_recording(
        &state,
        "播放源已切换，录制已自动停止",
    )?;
    // Switching source must stop any active decode stream first, otherwise the old
    // stream can keep emitting progress and consume resources.
    stop_decode_stream_non_blocking(&state)?;
    {
        let mut playback = state_locks::playback(&state)?;
        playback.open(path.clone());
    }
    *state_locks::latest_stream_position_seconds(&state)? = 0.0;
    *state_locks::pending_seek_seconds(&state)? = Some(0.0);
    {
        let mut library = state_locks::library(&state)?;
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
    stop_decode_stream_non_blocking(&state)?;
    sync_pause_resume_position(&state)?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn stop(
    app: AppHandle,
    state: State<'_, MediaState>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    finalize_active_cache_recording(
        &state,
        "播放已停止，录制已自动停止",
    )?;
    stop_decode_stream_non_blocking(&state)?;
    *state_locks::latest_stream_position_seconds(&state)? = 0.0;
    let current_path = {
        let mut playback = state_locks::playback(&state)?;
        let path = playback.state().current_path;
        playback.stop();
        path
    };
    *state_locks::pending_seek_seconds(&state)? = Some(0.0);
    if let Some(source) = current_path {
        // Do not block stop() on preview decode, especially for network streams (m3u8),
        // where opening/reading can take noticeable time and freezes the UI while the
        // Tauri command is awaiting completion.
        let app_handle = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let renderer = (*app_handle.state::<RendererState>()).clone();
            let media_state = app_handle.state::<MediaState>();
            if let Err(err) = viewport_sync::sync_main_viewport_to(&*media_state, &renderer, &source, 0.0) {
                eprintln!("stop preview failed: {err}");
            }
        });
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
    let position_seconds = normalize_non_negative(position_seconds, "position_seconds")?;
    let (media_path, status) = {
        let mut playback = state_locks::playback(&state)?;
        playback.seek(position_seconds);
        let playback_state = playback.state();
        (playback_state.current_path, playback_state.status)
    };
    if let Some(path_ref) = media_path.as_deref() {
        let mut library = state_locks::library(&state)?;
        library.mark_playback_progress(path_ref, position_seconds);
    }
    set_pending_seek(&state, position_seconds)?;
    if status == PlaybackStatus::Paused {
        if let Some(source) = media_path {
            let target = position_seconds.max(0.0);
            let renderer = (*app.state::<RendererState>()).clone();
            if let Err(err) = viewport_sync::sync_main_viewport_to(&state, &renderer, &source, target) {
                eprintln!("paused seek preview failed: {err}");
            }
        }
    }
    write_latest_stream_position(&state, position_seconds.max(0.0))?;
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playback_rate = normalize_playback_rate(playback_rate)?;
    {
        let mut playback = state_locks::playback(&state)?;
        playback.set_rate(playback_rate);
    }
    state.timing_controls.set_playback_rate(playback_rate as f32);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let volume = normalize_unit_interval(volume, "volume")?;
    state.audio_controls.set_volume(volume as f32);
    if volume <= 0.0 {
        state.audio_controls.set_muted(true);
    } else {
        state.audio_controls.set_muted(false);
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    state.audio_controls.set_muted(muted);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_hw_decode_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: HardwareDecodeMode,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playing_path = {
        let mut playback = state_locks::playback(&state)?;
        playback.set_hw_decode_mode(mode);
        // 新模式要等下一次打开 codec 才生效；仅改状态会沿用已存在的解码线程。
        playback.update_hw_decode_status(false, None, None);
        let st = playback.state();
        if st.status == PlaybackStatus::Playing {
            st.current_path.clone()
        } else {
            None
        }
    };
    if let Some(path) = playing_path {
        let latest = read_latest_stream_position(&state).unwrap_or(0.0);
        let resume = {
            let mut playback = state_locks::playback(&state)?;
            let pos = playback.state().position_seconds.max(latest).max(0.0);
            playback.seek(pos);
            pos
        };
        set_pending_seek(&state, resume)?;
        stop_decode_stream_blocking(&state)?;
        start_decode_stream(&app, &state, path)?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_quality_mode(
    app: AppHandle,
    state: State<'_, MediaState>,
    mode: PlaybackQualityMode,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playing_path = {
        let mut playback = state_locks::playback(&state)?;
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

    if let Some(path) = playing_path {
        let latest = read_latest_stream_position(&state).unwrap_or(0.0);
        let resume = {
            let mut playback = state_locks::playback(&state)?;
            let pos = playback.state().position_seconds.max(latest).max(0.0);
            playback.seek(pos);
            pos
        };
        set_pending_seek(&state, resume)?;
        stop_decode_stream_blocking(&state)?;
        start_decode_stream(&app, &state, path)?;
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let position_seconds = normalize_non_negative(position_seconds, "position_seconds")?;
    let duration_seconds = normalize_non_negative(duration_seconds, "duration_seconds")?;
    let path = {
        let mut playback = state_locks::playback(&state)?;
        playback.sync_position(position_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state_locks::library(&state)?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub async fn preview_frame(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> MediaResult<Option<PreviewFrame>> {
    let target = normalize_non_negative(position_seconds, "position_seconds")?;
    let source = {
        let mut playback = state_locks::playback(&state)?;
        playback.state().current_path
    };
    let Some(source) = source else {
        return Ok(None);
    };

    let epoch = state.preview_frame_epoch.fetch_add(1, Ordering::Relaxed) + 1;
    let width = normalize_preview_edge(max_width.unwrap_or(160));
    let height = normalize_preview_edge(max_height.unwrap_or(90));
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
    .map_err(|err| MediaError::internal(format!("preview task join failed: {err}")))?
    .map_err(MediaError::from)
}

pub fn get_cache_recording_status(state: State<'_, MediaState>) -> MediaResult<CacheRecordingStatus> {
    let guard = state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    if let Some(session) = guard.as_ref() {
        let output_size_bytes = fs::metadata(&session.output_path).ok().map(|meta| meta.len());
        Ok(CacheRecordingStatus {
            recording: session.active,
            source: Some(session.source.clone()),
            output_path: Some(session.output_path.clone()),
            finalized_output_path: (!session.active).then(|| session.output_path.clone()),
            output_size_bytes,
            started_at_ms: Some(session.started_at_ms),
            error_message: session.error_message.clone(),
            fallback_transcoding: Some(session.fallback_transcoding),
        })
    } else {
        Ok(CacheRecordingStatus {
            recording: false,
            source: None,
            output_path: None,
            finalized_output_path: None,
            output_size_bytes: None,
            started_at_ms: None,
            error_message: None,
            fallback_transcoding: None,
        })
    }
}

pub fn start_cache_recording(
    state: State<'_, MediaState>,
    output_dir: Option<String>,
) -> MediaResult<CacheRecordingStatus> {
    let source = {
        let mut playback = state_locks::playback(&state)?;
        let current = playback.state().current_path;
        current.ok_or_else(|| MediaError::invalid_input("no active source to cache"))?
    };

    let output_base_dir = output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| MediaError::invalid_input("cache output_dir is required"))?;
    fs::create_dir_all(&output_base_dir).map_err(|err| {
        MediaError::internal(format!(
            "failed to create cache output directory '{}': {err}",
            output_base_dir
        ))
    })?;
    let started_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let source_lower = source.to_ascii_lowercase();
    let is_live_source = source_lower.contains(".m3u8") || is_network_source(&source_lower);
    let output_ext = if is_live_source { "ts" } else { "mp4" };
    let output_path = format!(
        "{}/mediax-cache-{}.{}",
        output_base_dir.trim_end_matches('/'),
        started_at_ms,
        output_ext
    );

    let mut recorder_guard = state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    if recorder_guard.is_some() {
        return Err(MediaError::invalid_input("cache recording already running"));
    }

    if source_lower.ends_with(".mp4") && source_lower.contains("mediax-cache-") {
        return Err(MediaError::invalid_input(
            "cannot start cache recording from an existing cache recording file",
        ));
    }
    if source_lower.ends_with(".ts") && source_lower.contains("mediax-cache-") {
        return Err(MediaError::invalid_input(
            "cannot start cache recording from an existing cache recording file",
        ));
    }

    *recorder_guard = Some(CacheRecorderSession {
        source: source.clone(),
        output_path: output_path.clone(),
        started_at_ms,
        active: true,
        fallback_transcoding: false,
        error_message: None,
    });
    drop(recorder_guard);

    Ok(CacheRecordingStatus {
        recording: true,
        source: Some(source),
        output_path: Some(output_path),
        finalized_output_path: None,
        output_size_bytes: Some(0),
        started_at_ms: Some(started_at_ms),
        error_message: None,
        fallback_transcoding: Some(false),
    })
}

pub fn stop_cache_recording(state: State<'_, MediaState>) -> MediaResult<CacheRecordingStatus> {
    let mut recorder_guard = state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    let Some(session) = recorder_guard.take() else {
        return Ok(CacheRecordingStatus {
            recording: false,
            source: None,
            output_path: None,
            finalized_output_path: None,
            output_size_bytes: None,
            started_at_ms: None,
            error_message: None,
            fallback_transcoding: None,
        });
    };
    let output_size_bytes = fs::metadata(&session.output_path).ok().map(|meta| meta.len());
    Ok(CacheRecordingStatus {
        recording: false,
        source: Some(session.source),
        output_path: Some(session.output_path.clone()),
        finalized_output_path: Some(session.output_path),
        output_size_bytes,
        started_at_ms: Some(session.started_at_ms),
        error_message: session.error_message,
        fallback_transcoding: Some(session.fallback_transcoding),
    })
}

fn finalize_active_cache_recording(
    state: &State<'_, MediaState>,
    reason: &str,
) -> MediaResult<()> {
    let mut recorder_guard = state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    let Some(session) = recorder_guard.as_mut() else {
        return Ok(());
    };
    if !session.active {
        return Ok(());
    }
    session.active = false;
    session.error_message = Some(reason.to_string());
    Ok(())
}

fn is_network_source(source: &str) -> bool {
    source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("rtsp://")
        || source.starts_with("rtmp://")
        || source.starts_with("mms://")
}

fn set_pending_seek(state: &State<'_, MediaState>, position_seconds: f64) -> MediaResult<()> {
    *state_locks::pending_seek_seconds(state)? = Some(position_seconds.max(0.0));
    Ok(())
}

fn normalize_non_negative(value: f64, field: &str) -> MediaResult<f64> {
    if !value.is_finite() {
        return Err(MediaError::invalid_input(format!(
            "{field} must be a finite number"
        )));
    }
    Ok(value.max(0.0))
}

fn normalize_unit_interval(value: f64, field: &str) -> MediaResult<f64> {
    if !value.is_finite() {
        return Err(MediaError::invalid_input(format!(
            "{field} must be a finite number"
        )));
    }
    Ok(value.clamp(0.0, 1.0))
}

fn normalize_playback_rate(playback_rate: f64) -> MediaResult<f64> {
    if !playback_rate.is_finite() {
        return Err(MediaError::invalid_input(
            "playback_rate must be a finite number",
        ));
    }
    Ok(playback_rate.clamp(MIN_PLAYBACK_RATE, MAX_PLAYBACK_RATE))
}

fn normalize_preview_edge(edge: u32) -> u32 {
    edge.clamp(MIN_PREVIEW_EDGE, MAX_PREVIEW_EDGE)
}

fn activate_playback_and_resume_position(
    state: &State<'_, MediaState>,
) -> MediaResult<(Option<String>, f64)> {
    let latest_stream_position_seconds = read_latest_stream_position(state).unwrap_or(0.0);
    let mut playback = state_locks::playback(state)?;
    playback.play();
    let playback_state = playback.state();
    let resume_position_seconds = playback_state
        .position_seconds
        .max(latest_stream_position_seconds)
        .max(0.0);
    playback.seek(resume_position_seconds);
    Ok((playback_state.current_path, resume_position_seconds))
}

fn sync_pause_resume_position(state: &State<'_, MediaState>) -> MediaResult<()> {
    let latest_stream_position_seconds = read_latest_stream_position(state).unwrap_or(0.0);
    let mut playback = state_locks::playback(state)?;
    let current_position_seconds = playback.state().position_seconds.max(0.0);
    let resume_position_seconds = current_position_seconds.max(latest_stream_position_seconds);
    playback.seek(resume_position_seconds);
    playback.pause();
    Ok(())
}
