use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::session::constraints;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, State};

pub fn set_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let volume = constraints::normalize_unit_interval(volume, "volume")?;
    {
        let mut playback = state::playback(&state)?;
        playback.set_volume(volume);
    }
    state.controls.audio.set_volume(volume as f32);
    if volume <= 0.0 {
        state.controls.audio.set_muted(true);
    } else {
        state.controls.audio.set_muted(false);
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    {
        let mut playback = state::playback(&state)?;
        playback.set_muted(muted);
    }
    state.controls.audio.set_muted(muted);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_left_channel_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let volume = constraints::normalize_unit_interval(volume, "volume")?;
    {
        let mut playback = state::playback(&state)?;
        playback.set_left_channel_volume(volume);
    }
    state.controls.audio.set_left_volume(volume as f32);
    state.controls.audio.set_left_muted(volume <= 0.0);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_right_channel_volume(
    app: AppHandle,
    state: State<'_, MediaState>,
    volume: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let volume = constraints::normalize_unit_interval(volume, "volume")?;
    {
        let mut playback = state::playback(&state)?;
        playback.set_right_channel_volume(volume);
    }
    state.controls.audio.set_right_volume(volume as f32);
    state.controls.audio.set_right_muted(volume <= 0.0);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}
