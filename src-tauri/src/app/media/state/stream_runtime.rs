use crate::app::media::error::MediaError;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Default)]
pub struct StreamRuntimeState {
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    pending_seek_seconds: Mutex<Option<f64>>,
    latest_position_seconds: Mutex<f64>,
    generation: AtomicU32,
}

pub type DecodeStreamHandles = (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>);

impl StreamRuntimeState {
    pub fn take_decode_stream_handles(&self) -> Result<DecodeStreamHandles, MediaError> {
        let stop_flag = self
            .stop_flag
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream state"))?
            .take();
        let thread = self
            .thread
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream thread"))?
            .take();
        Ok((stop_flag, thread))
    }

    pub fn install_decode_stream_handle(
        &self,
        stop_flag: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Result<(), MediaError> {
        *self
            .stop_flag
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream state"))? = Some(stop_flag);
        *self
            .thread
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream thread"))? = Some(thread);
        Ok(())
    }

    pub fn has_active_stream(&self) -> Result<bool, MediaError> {
        let has_stop_flag = self
            .stop_flag
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream state"))?
            .is_some();
        let has_thread = self
            .thread
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("stream thread"))?
            .is_some();
        Ok(has_stop_flag || has_thread)
    }

    pub fn request_stop(handles: &DecodeStreamHandles) {
        if let Some(flag) = handles.0.as_ref() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn join(handles: DecodeStreamHandles) {
        if let Some(handle) = handles.1 {
            let _ = handle.join();
        }
    }

    pub fn set_latest_position_seconds(&self, position_seconds: f64) -> Result<(), MediaError> {
        *self
            .latest_position_seconds
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("latest position state"))? =
            position_seconds.max(0.0);
        Ok(())
    }

    pub fn latest_position_seconds(&self) -> Result<f64, MediaError> {
        let value = *self
            .latest_position_seconds
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("latest position state"))?;
        Ok(value.max(0.0))
    }

    pub fn advance_generation(&self) -> u32 {
        self.generation
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    pub fn current_generation(&self) -> u32 {
        self.generation.load(Ordering::Relaxed)
    }

    pub fn is_generation_current(&self, generation: u32) -> bool {
        self.current_generation() == generation
    }

    pub fn set_pending_seek_seconds(&self, position_seconds: f64) -> Result<(), MediaError> {
        *self
            .pending_seek_seconds
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))? =
            Some(position_seconds.max(0.0));
        Ok(())
    }

    pub fn reset_pending_seek_to_zero(&self) -> Result<(), MediaError> {
        *self
            .pending_seek_seconds
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))? = Some(0.0);
        Ok(())
    }

    pub fn take_pending_seek_seconds(&self) -> Result<Option<f64>, MediaError> {
        Ok(self
            .pending_seek_seconds
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("pending seek state"))?
            .take())
    }
}
