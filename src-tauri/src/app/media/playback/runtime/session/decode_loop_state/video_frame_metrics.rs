use std::time::Instant;

pub(crate) struct VideoTimestampMetrics {
    pub window_start: Instant,
    pub samples: u64,
    pub pts_missing: u64,
    pub pts_backtrack: u64,
    pub pts_jitter_abs_sum_ms: f64,
    pub pts_jitter_max_ms: f64,
    pub last_gap_seconds: Option<f64>,
}

impl VideoTimestampMetrics {
    pub fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            samples: 0,
            pts_missing: 0,
            pts_backtrack: 0,
            pts_jitter_abs_sum_ms: 0.0,
            pts_jitter_max_ms: 0.0,
            last_gap_seconds: None,
        }
    }
}

pub(crate) struct VideoFrameTypeMetrics {
    pub window_start: Instant,
    pub i_count: u64,
    pub p_count: u64,
    pub b_count: u64,
    pub other_count: u64,
}

impl VideoFrameTypeMetrics {
    pub fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            i_count: 0,
            p_count: 0,
            b_count: 0,
            other_count: 0,
        }
    }
}
