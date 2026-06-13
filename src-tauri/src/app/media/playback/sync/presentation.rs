/// Source-frame multiples used for presentation timing. One frame of display latency is the
/// standard model for GPU/compositor delay without per-file tuning.
const DISPLAY_LATENCY_FRAMES: f64 = 1.0;
const STALE_LAG_FRAMES: f64 = 2.5;
const MIN_FRAME_DURATION_SECONDS: f64 = 1.0 / 120.0;
const MAX_FRAME_DURATION_SECONDS: f64 = 1.0 / 10.0;

#[derive(Clone, Copy, Debug)]
pub struct PresentationPolicy {
    source_frame_duration_seconds: f64,
}

impl PresentationPolicy {
    pub fn from_frame_duration(frame_duration_seconds: f64) -> Self {
        let safe = if frame_duration_seconds.is_finite() && frame_duration_seconds > 0.0 {
            frame_duration_seconds
        } else {
            1.0 / 24.0
        };
        Self {
            source_frame_duration_seconds: safe.clamp(
                MIN_FRAME_DURATION_SECONDS,
                MAX_FRAME_DURATION_SECONDS,
            ),
        }
    }

    pub fn presentation_deadline(&self, master_seconds: f64) -> f64 {
        master_seconds + self.display_latency_seconds()
    }

    /// Drop queued frames older than this PTS — recover from stalls without large jumps.
    pub fn stale_threshold(&self, master_seconds: f64) -> f64 {
        master_seconds - self.stale_lag_seconds()
    }

    fn display_latency_seconds(&self) -> f64 {
        self.source_frame_duration_seconds * DISPLAY_LATENCY_FRAMES
    }

    fn stale_lag_seconds(&self) -> f64 {
        (self.source_frame_duration_seconds * STALE_LAG_FRAMES).clamp(0.050, 0.200)
    }
}
