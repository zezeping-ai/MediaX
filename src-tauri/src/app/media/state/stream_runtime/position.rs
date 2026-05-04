use crate::app::media::error::MediaError;

use super::StreamRuntimeState;

pub(super) fn set_latest_position_seconds(
    state: &StreamRuntimeState,
    position_seconds: f64,
) -> Result<(), MediaError> {
    *state
        .latest_position_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("latest position state"))? =
        position_seconds.max(0.0);
    Ok(())
}

pub(super) fn latest_position_seconds(state: &StreamRuntimeState) -> Result<f64, MediaError> {
    let value = *state
        .latest_position_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("latest position state"))?;
    Ok(value.max(0.0))
}
