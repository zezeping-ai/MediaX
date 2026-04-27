use super::value::PlaybackRate;
use std::sync::atomic::{AtomicU32, Ordering};

pub struct TimingControls {
    playback_rate_bits: AtomicU32,
}

impl Default for TimingControls {
    fn default() -> Self {
        Self {
            playback_rate_bits: AtomicU32::new(PlaybackRate::default().as_f32().to_bits()),
        }
    }
}

impl TimingControls {
    pub fn playback_rate(&self) -> f32 {
        let bits = self.playback_rate_bits.load(Ordering::Relaxed);
        PlaybackRate::new(f32::from_bits(bits)).as_f32()
    }

    pub fn playback_rate_value(&self) -> PlaybackRate {
        let bits = self.playback_rate_bits.load(Ordering::Relaxed);
        PlaybackRate::new(f32::from_bits(bits))
    }

    pub fn set_playback_rate(&self, value: f32) {
        self.set_playback_rate_value(PlaybackRate::new(value));
    }

    pub fn set_playback_rate_value(&self, value: PlaybackRate) {
        self.playback_rate_bits
            .store(value.as_f32().to_bits(), Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::TimingControls;

    #[test]
    fn timing_controls_default_to_one_x() {
        let controls = TimingControls::default();
        assert!((controls.playback_rate() - 1.0).abs() <= 1e-6);
    }
}
