pub const MIN_PLAYBACK_RATE: f32 = 0.25;
pub const MAX_PLAYBACK_RATE: f32 = 3.0;
pub const DEFAULT_PLAYBACK_RATE: f32 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PlaybackRate(f32);

impl PlaybackRate {
    pub fn new(value: f32) -> Self {
        let normalized = if value.is_finite() && value > 0.0 {
            value.clamp(MIN_PLAYBACK_RATE, MAX_PLAYBACK_RATE)
        } else {
            DEFAULT_PLAYBACK_RATE
        };
        Self(normalized)
    }

    pub fn from_f64(value: f64) -> Self {
        Self::new(value as f32)
    }

    pub fn as_f32(self) -> f32 {
        self.0
    }

    pub fn as_f64(self) -> f64 {
        f64::from(self.0)
    }

    pub fn is_neutral(self) -> bool {
        (self.0 - DEFAULT_PLAYBACK_RATE).abs() <= 1e-3
    }

    pub fn delta(self, other: Self) -> f32 {
        (self.0 - other.0).abs()
    }
}

impl Default for PlaybackRate {
    fn default() -> Self {
        Self(DEFAULT_PLAYBACK_RATE)
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackRate, DEFAULT_PLAYBACK_RATE};

    #[test]
    fn non_positive_values_fall_back_to_default_rate() {
        assert_eq!(PlaybackRate::new(0.0).as_f32(), DEFAULT_PLAYBACK_RATE);
        assert_eq!(PlaybackRate::new(-1.0).as_f32(), DEFAULT_PLAYBACK_RATE);
    }
}
