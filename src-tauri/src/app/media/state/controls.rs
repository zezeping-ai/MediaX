use crate::app::media::playback::dto::PlaybackChannelRouting;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

#[derive(Default)]
pub struct AudioControls {
    volume_bits: AtomicU32,
    muted: AtomicBool,
    left_volume_bits: AtomicU32,
    right_volume_bits: AtomicU32,
    left_muted: AtomicBool,
    right_muted: AtomicBool,
    channel_routing: AtomicU8,
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

    pub fn channel_routing(&self) -> PlaybackChannelRouting {
        match self.channel_routing.load(Ordering::Relaxed) {
            1 => PlaybackChannelRouting::LeftToBoth,
            2 => PlaybackChannelRouting::RightToBoth,
            _ => PlaybackChannelRouting::Stereo,
        }
    }

    pub fn set_channel_routing(&self, value: PlaybackChannelRouting) {
        let encoded = match value {
            PlaybackChannelRouting::Stereo => 0,
            PlaybackChannelRouting::LeftToBoth => 1,
            PlaybackChannelRouting::RightToBoth => 2,
        };
        self.channel_routing.store(encoded, Ordering::Relaxed);
    }
}
