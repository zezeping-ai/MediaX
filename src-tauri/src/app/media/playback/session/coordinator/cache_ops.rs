use super::helpers::{create_cache_recorder_session, is_network_source};
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::CacheRecordingStatus;
use crate::app::media::playback::events::{build_media_event, MEDIA_CACHE_RECORDING_STATUS_EVENT};
use crate::app::media::state;
use crate::app::media::state::MediaState;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

pub fn get_cache_recording_status(
    state: State<'_, MediaState>,
) -> MediaResult<CacheRecordingStatus> {
    cache_recording_status_from_state(&state)
}

pub fn cache_recording_status_from_state(
    state: &State<'_, MediaState>,
) -> MediaResult<CacheRecordingStatus> {
    let guard = state
        .cache
        .recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    if let Some(session) = guard.as_ref() {
        let output_size_bytes = fs::metadata(&session.output_path)
            .ok()
            .map(|meta| meta.len());
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

pub fn emit_cache_recording_status(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    force_emit_inactive: bool,
) -> MediaResult<()> {
    let status = cache_recording_status_from_state(state)?;
    if !force_emit_inactive && !status.recording {
        return Ok(());
    }
    app.emit(
        MEDIA_CACHE_RECORDING_STATUS_EVENT,
        build_media_event("cache_recording_status", None, status),
    )
    .map_err(|err| MediaError::internal(format!("emit cache recording status failed: {err}")))?;
    Ok(())
}

pub fn start_cache_recording(
    app: AppHandle,
    state: State<'_, MediaState>,
    output_dir: Option<String>,
) -> MediaResult<CacheRecordingStatus> {
    let source = {
        let playback = state::playback(&state)?;
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
        .cache
        .recorder
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

    *recorder_guard = Some(create_cache_recorder_session(
        source.clone(),
        output_path.clone(),
        started_at_ms,
    ));
    drop(recorder_guard);

    let status = CacheRecordingStatus {
        recording: true,
        source: Some(source),
        output_path: Some(output_path),
        finalized_output_path: None,
        output_size_bytes: Some(0),
        started_at_ms: Some(started_at_ms),
        error_message: None,
        fallback_transcoding: Some(false),
    };
    let _ = emit_cache_recording_status(&app, &state, true);
    Ok(status)
}

pub fn stop_cache_recording(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> MediaResult<CacheRecordingStatus> {
    let mut recorder_guard = state
        .cache
        .recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder"))?;
    let Some(session) = recorder_guard.take() else {
        let status = CacheRecordingStatus {
            recording: false,
            source: None,
            output_path: None,
            finalized_output_path: None,
            output_size_bytes: None,
            started_at_ms: None,
            error_message: None,
            fallback_transcoding: None,
        };
        let _ = app.emit(
            MEDIA_CACHE_RECORDING_STATUS_EVENT,
            build_media_event("cache_recording_status", None, status.clone()),
        );
        return Ok(status);
    };
    let output_size_bytes = fs::metadata(&session.output_path)
        .ok()
        .map(|meta| meta.len());
    let status = CacheRecordingStatus {
        recording: false,
        source: Some(session.source),
        output_path: Some(session.output_path.clone()),
        finalized_output_path: Some(session.output_path),
        output_size_bytes,
        started_at_ms: Some(session.started_at_ms),
        error_message: session.error_message,
        fallback_transcoding: Some(session.fallback_transcoding),
    };
    let _ = app.emit(
        MEDIA_CACHE_RECORDING_STATUS_EVENT,
        build_media_event("cache_recording_status", None, status.clone()),
    );
    Ok(status)
}
