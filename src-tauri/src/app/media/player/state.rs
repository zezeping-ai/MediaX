use crate::app::media::library::MediaLibraryService;
use crate::app::media::player::playback::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread::JoinHandle;

#[derive(Default)]
pub struct StreamRuntimeState {
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    pending_seek_seconds: Mutex<Option<f64>>,
    latest_position_seconds: Mutex<f64>,
}

pub type DecodeStreamHandles = (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>);

impl StreamRuntimeState {
    pub fn take_decode_stream_handles(
        &self,
    ) -> Result<DecodeStreamHandles, crate::app::media::error::MediaError> {
        let stop_flag = self
            .stop_flag
            .lock()
            .map_err(|_| crate::app::media::error::MediaError::state_poisoned_lock("stream state"))?
            .take();
        let thread = self
            .thread
            .lock()
            .map_err(|_| {
                crate::app::media::error::MediaError::state_poisoned_lock("stream thread")
            })?
            .take();
        Ok((stop_flag, thread))
    }

    pub fn install_decode_stream_handle(
        &self,
        stop_flag: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.stop_flag.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("stream state")
        })? = Some(stop_flag);
        *self.thread.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("stream thread")
        })? = Some(thread);
        Ok(())
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

    pub fn set_latest_position_seconds(
        &self,
        position_seconds: f64,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.latest_position_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("latest position state")
        })? = position_seconds.max(0.0);
        Ok(())
    }

    pub fn latest_position_seconds(&self) -> Result<f64, crate::app::media::error::MediaError> {
        let value = *self.latest_position_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("latest position state")
        })?;
        Ok(value.max(0.0))
    }

    pub fn set_pending_seek_seconds(
        &self,
        position_seconds: f64,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.pending_seek_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
        })? = Some(position_seconds.max(0.0));
        Ok(())
    }

    pub fn reset_pending_seek_to_zero(&self) -> Result<(), crate::app::media::error::MediaError> {
        *self.pending_seek_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
        })? = Some(0.0);
        Ok(())
    }

    pub fn take_pending_seek_seconds(
        &self,
    ) -> Result<Option<f64>, crate::app::media::error::MediaError> {
        Ok(self
            .pending_seek_seconds
            .lock()
            .map_err(|_| {
                crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
            })?
            .take())
    }
}

pub struct CacheRecorderSession {
    pub source: String,
    pub output_path: String,
    pub started_at_ms: u64,
    pub active: bool,
    pub fallback_transcoding: bool,
    pub error_message: Option<String>,
}

#[derive(Default)]
pub struct MediaState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
    pub stream: StreamRuntimeState,
    // Cancellation epoch for paused seek rendering to main viewport.
    pub paused_seek_epoch: AtomicU32,
    // Cancellation epoch for timeline hover thumbnail generation.
    pub preview_frame_epoch: AtomicU32,
    pub audio_controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
    pub cache_recorder: Mutex<Option<CacheRecorderSession>>,
}

#[derive(Default)]
pub struct AudioControls {
    // Store IEEE-754 bits for lock-free reads in decode loop.
    volume_bits: AtomicU32,
    muted: AtomicBool,
}

#[derive(Default)]
pub struct TimingControls {
    // Store IEEE-754 bits for lock-free reads in decode loop.
    playback_rate_bits: AtomicU32,
}

impl AudioControls {
    pub fn volume(&self) -> f32 {
        let bits = self.volume_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.min(1.0)
        } else {
            1.0
        }
    }

    pub fn set_volume(&self, value: f32) {
        let normalized = value.clamp(0.0, 1.0);
        self.volume_bits
            .store(normalized.to_bits(), Ordering::Relaxed);
    }

    pub fn muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn set_muted(&self, value: bool) {
        self.muted.store(value, Ordering::Relaxed);
    }
}

impl TimingControls {
    pub fn playback_rate(&self) -> f32 {
        let bits = self.playback_rate_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.clamp(0.25, 3.0)
        } else {
            1.0
        }
    }

    pub fn set_playback_rate(&self, value: f32) {
        let normalized = value.clamp(0.25, 3.0);
        self.playback_rate_bits
            .store(normalized.to_bits(), Ordering::Relaxed);
    }
}
