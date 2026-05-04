use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, State};

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
