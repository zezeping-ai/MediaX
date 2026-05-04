#[derive(Default)]
pub(crate) struct AudioStats {
    pub packets: u64,
    pub decoded_frames: u64,
    pub queued_samples: u64,
    pub underrun_count: u64,
    pub intentional_refill_pending: bool,
    pub seek_refill_logged: bool,
    pub audio_only_backpressure_logged: bool,
    pub decode_supply_gap_logged: bool,
    pub low_queue_logged: bool,
    pub low_queue_log_counter: u64,
}
