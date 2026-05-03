pub(crate) const DECODE_LEAD_SLEEP_MS: u64 = 5;
// Frame-slot availability is signaled via Condvar, so this timeout is only a fallback
// for stop/shutdown responsiveness. Keep it coarse to avoid decode-thread busy wakeups
// while the renderer queue is intentionally full.
pub(super) const RENDER_BACKPRESSURE_SLEEP_MS: u64 = 12;
