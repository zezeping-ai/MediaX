use crate::app::media::error::MediaError;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

mod epochs;
mod handles;
mod pending_seek;
mod position;

pub type DecodeStreamHandles = (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>);

#[derive(Default)]
pub struct StreamRuntimeState {
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    pending_seek_seconds: Mutex<Option<f64>>,
    latest_position_seconds: Mutex<f64>,
    generation: AtomicU32,
    restart_epoch: AtomicU64,
}

impl StreamRuntimeState {
    pub fn next_restart_epoch(&self) -> u64 {
        epochs::next_restart_epoch(self)
    }

    pub fn is_restart_epoch_current(&self, epoch: u64) -> bool {
        epochs::is_restart_epoch_current(self, epoch)
    }

    pub fn take_decode_stream_handles(&self) -> Result<DecodeStreamHandles, MediaError> {
        handles::take_decode_stream_handles(self)
    }

    pub fn install_decode_stream_handle(
        &self,
        stop_flag: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Result<(), MediaError> {
        handles::install_decode_stream_handle(self, stop_flag, thread)
    }

    pub fn has_active_stream(&self) -> Result<bool, MediaError> {
        handles::has_active_stream(self)
    }

    pub fn request_stop(handles: &DecodeStreamHandles) {
        handles::request_stop(handles)
    }

    pub fn set_latest_position_seconds(&self, position_seconds: f64) -> Result<(), MediaError> {
        position::set_latest_position_seconds(self, position_seconds)
    }

    pub fn latest_position_seconds(&self) -> Result<f64, MediaError> {
        position::latest_position_seconds(self)
    }

    pub fn advance_generation(&self) -> u32 {
        epochs::advance_generation(self)
    }

    pub fn current_generation(&self) -> u32 {
        epochs::current_generation(self)
    }

    pub fn is_generation_current(&self, generation: u32) -> bool {
        self.current_generation() == generation
    }

    pub fn set_pending_seek_seconds(&self, position_seconds: f64) -> Result<(), MediaError> {
        pending_seek::set_pending_seek_seconds(self, position_seconds)
    }

    pub fn reset_pending_seek_to_zero(&self) -> Result<(), MediaError> {
        pending_seek::reset_pending_seek_to_zero(self)
    }

    pub fn take_pending_seek_seconds(&self) -> Result<Option<f64>, MediaError> {
        pending_seek::take_pending_seek_seconds(self)
    }
}
