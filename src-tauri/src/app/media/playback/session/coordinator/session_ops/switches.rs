use super::restart::restart_active_playback;
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::{HardwareDecodeMode, PlaybackQualityMode, PlaybackStatus};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, State};

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
    emit_debug(&app, "quality_request", format!("requested quality_mode={mode:?}"));
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
