use crate::app::media::error::MediaError;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

use super::{DecodeStreamHandles, StreamRuntimeState};

pub(super) fn take_decode_stream_handles(
    state: &StreamRuntimeState,
) -> Result<DecodeStreamHandles, MediaError> {
    let stop_flag = state
        .stop_flag
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream state"))?
        .take();
    let thread = state
        .thread
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream thread"))?
        .take();
    Ok((stop_flag, thread))
}

pub(super) fn install_decode_stream_handle(
    state: &StreamRuntimeState,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
    thread: JoinHandle<()>,
) -> Result<(), MediaError> {
    *state
        .stop_flag
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream state"))? = Some(stop_flag);
    *state
        .thread
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream thread"))? = Some(thread);
    Ok(())
}

pub(super) fn has_active_stream(state: &StreamRuntimeState) -> Result<bool, MediaError> {
    let has_stop_flag = state
        .stop_flag
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream state"))?
        .is_some();
    let has_thread = state
        .thread
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("stream thread"))?
        .is_some();
    Ok(has_stop_flag || has_thread)
}

pub(super) fn request_stop(handles: &DecodeStreamHandles) {
    if let Some(flag) = handles.0.as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}
