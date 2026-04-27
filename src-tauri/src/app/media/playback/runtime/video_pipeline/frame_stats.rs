use std::time::Instant;

pub(crate) struct VideoTimestampMetricsRef<'a> {
    pub window_start: &'a mut Instant,
    pub samples: &'a mut u64,
    pub pts_missing: &'a mut u64,
    pub pts_backtrack: &'a mut u64,
    pub pts_jitter_abs_sum_ms: &'a mut f64,
    pub pts_jitter_max_ms: &'a mut f64,
    pub last_gap_seconds: &'a mut Option<f64>,
}

pub(crate) struct VideoFrameTypeMetricsRef<'a> {
    pub window_start: &'a mut Instant,
    pub i_count: &'a mut u64,
    pub p_count: &'a mut u64,
    pub b_count: &'a mut u64,
    pub other_count: &'a mut u64,
}
