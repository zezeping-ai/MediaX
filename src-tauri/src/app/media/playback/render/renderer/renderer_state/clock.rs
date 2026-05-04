use super::{ClockState, RendererInner};
use std::time::Instant;

pub(super) fn current_clock_seconds(inner: &RendererInner) -> f64 {
    let clock = match inner.clock.lock() {
        Ok(guard) => *guard,
        Err(_) => ClockState {
            anchor_instant: Instant::now(),
            anchor_media_seconds: 0.0,
            rate: 1.0,
        },
    };
    let elapsed = Instant::now().saturating_duration_since(clock.anchor_instant);
    clock.anchor_media_seconds + elapsed.as_secs_f64() * clock.rate.max(0.25)
}

