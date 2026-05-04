use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::runtime::emit_debug;
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
    emit_debug(
        &app,
        "rate_request",
        format!("requested playback_rate={playback_rate:.3}"),
    );
    let playback_rate = constraints::normalize_playback_rate(playback_rate)?;
    {
        let mut playback = state::playback(&state)?;
        playback.set_rate(playback_rate.as_f64());
    }
    state
        .controls
        .timing
        .set_playback_rate(playback_rate.as_f32());
    emit_debug(
        &app,
        "rate_apply",
        format!("session playback_rate set to {:.3}", playback_rate.as_f64()),
    );
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}
