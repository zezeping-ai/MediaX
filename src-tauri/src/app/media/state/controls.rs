use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Default)]
pub struct AudioControls {
    volume_bits: AtomicU32,
    muted: AtomicBool,
}

#[derive(Default)]
pub struct TimingControls {
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
        self.volume_bits.store(normalized.to_bits(), Ordering::Relaxed);
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
