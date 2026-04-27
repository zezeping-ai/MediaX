use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::playback::session::constraints;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, State};

pub fn set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let playback_rate = constraints::normalize_playback_rate(playback_rate)?;
    {
        let mut playback = state::playback(&state)?;
        playback.set_rate(playback_rate.as_f64());
    }
    state
        .controls
        .timing
        .set_playback_rate(playback_rate.as_f32());
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

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

pub fn set_left_channel_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    {
        let mut playback = state::playback(&state)?;
        playback.set_left_channel_muted(muted);
    }
    state.controls.audio.set_left_muted(muted);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_right_channel_muted(
    app: AppHandle,
    state: State<'_, MediaState>,
    muted: bool,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    {
        let mut playback = state::playback(&state)?;
        playback.set_right_channel_muted(muted);
    }
    state.controls.audio.set_right_muted(muted);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn set_channel_routing(
    app: AppHandle,
    state: State<'_, MediaState>,
    routing: PlaybackChannelRouting,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    {
        let mut playback = state::playback(&state)?;
        playback.set_channel_routing(routing);
    }
    state.controls.audio.set_channel_routing(routing);
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

pub fn sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let position_seconds =
        constraints::normalize_non_negative(position_seconds, "position_seconds")?;
    let duration_seconds =
        constraints::normalize_non_negative(duration_seconds, "duration_seconds")?;
    let path = {
        let mut playback = state::playback(&state)?;
        playback.sync_position(position_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state::library(&state)?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}
