//! Tauri command orchestration: acquire [`MediaState`] locks, drive decode/runtime, sync viewport, emit snapshots.
//!
//! Heavy policy lives in [`crate::app::media::playback::render::viewport_sync`] and [`crate::app::media::playback::runtime`].

mod cache_ops;
mod helpers;
mod preview_ops;
mod session_ops;
mod timing_ops;

use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::state::MediaState;
use crate::app::media::state::snapshot_from_state;
use tauri::State;

pub use cache_ops::{
    get_cache_recording_status, start_cache_recording, stop_cache_recording,
};
pub use preview_ops::preview_frame;
pub use session_ops::{open, pause, play, seek, set_hw_decode_mode, set_quality_mode, stop};
pub use timing_ops::{
    set_left_channel_muted, set_left_channel_volume, set_muted, set_rate,
    set_right_channel_muted, set_right_channel_volume, set_volume, sync_position,
};

pub fn get_snapshot(state: State<'_, MediaState>) -> MediaResult<MediaSnapshot> {
    snapshot_from_state(&state).map_err(MediaError::from)
}
