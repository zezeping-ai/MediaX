use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Default)]
pub struct AudioControls {
    volume_bits: AtomicU32,
    muted: AtomicBool,
    left_volume_bits: AtomicU32,
    right_volume_bits: AtomicU32,
    left_muted: AtomicBool,
    right_muted: AtomicBool,
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

    pub fn left_volume(&self) -> f32 {
        let bits = self.left_volume_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.min(1.0)
        } else {
            1.0
        }
    }

    pub fn set_left_volume(&self, value: f32) {
        self.left_volume_bits
            .store(value.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn right_volume(&self) -> f32 {
        let bits = self.right_volume_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.min(1.0)
        } else {
            1.0
        }
    }

    pub fn set_right_volume(&self, value: f32) {
        self.right_volume_bits
            .store(value.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn left_muted(&self) -> bool {
        self.left_muted.load(Ordering::Relaxed)
    }

    pub fn set_left_muted(&self, value: bool) {
        self.left_muted.store(value, Ordering::Relaxed);
    }

    pub fn right_muted(&self) -> bool {
        self.right_muted.load(Ordering::Relaxed)
    }

    pub fn set_right_muted(&self, value: bool) {
        self.right_muted.store(value, Ordering::Relaxed);
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
