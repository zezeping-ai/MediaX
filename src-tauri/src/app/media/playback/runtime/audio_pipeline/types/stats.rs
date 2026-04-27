use std::time::Instant;

#[derive(Default)]
pub(crate) struct AudioStats {
    pub packets: u64,
    pub decoded_frames: u64,
    pub queued_samples: u64,
    pub underrun_count: u64,
    pub intentional_refill_pending: bool,
    pub audio_only_backpressure_logged: bool,
    pub last_debug_instant: Option<Instant>,
}
