use crate::app::media::library::MediaLibraryService;
use crate::app::media::player::playback::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
pub struct MediaState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
    pub stream_stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    pub pending_seek_seconds: Mutex<Option<f64>>,
    // Cancellation epoch for paused seek rendering to main viewport.
    pub paused_seek_epoch: AtomicU32,
    // Cancellation epoch for timeline hover thumbnail generation.
    pub preview_frame_epoch: AtomicU32,
    pub latest_stream_position_seconds: Mutex<f64>,
    pub audio_controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
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
