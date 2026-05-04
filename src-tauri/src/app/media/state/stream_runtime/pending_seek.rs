use crate::app::media::error::MediaError;

use super::StreamRuntimeState;

pub(super) fn set_pending_seek_seconds(
    state: &StreamRuntimeState,
    position_seconds: f64,
) -> Result<(), MediaError> {
    *state
        .pending_seek_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))? =
        Some(position_seconds.max(0.0));
    Ok(())
}

pub(super) fn reset_pending_seek_to_zero(state: &StreamRuntimeState) -> Result<(), MediaError> {
    *state
        .pending_seek_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))? = Some(0.0);
    Ok(())
}

pub(super) fn take_pending_seek_seconds(
    state: &StreamRuntimeState,
) -> Result<Option<f64>, MediaError> {
    Ok(state
        .pending_seek_seconds
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))?
        .take())
}
