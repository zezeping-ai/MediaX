use crate::app::media::playback::runtime::METRICS_EMIT_INTERVAL_MS;
use std::time::{Duration, Instant};

pub(crate) struct NetworkMetrics {
    window_start: Instant,
    window_bytes: u64,
    read_bps: Option<f64>,
    media_rate_window_start: Instant,
    media_rate_window_bytes: u64,
    media_required_bps: Option<f64>,
}

impl NetworkMetrics {
    pub fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            window_bytes: 0,
            read_bps: None,
            media_rate_window_start: now,
            media_rate_window_bytes: 0,
            media_required_bps: None,
        }
    }

    pub fn update_network_window(&mut self, packet_size: usize) {
        self.window_bytes = self.window_bytes.saturating_add(packet_size as u64);
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.window_start);
        if dt >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
            let seconds = dt.as_secs_f64().max(1e-6);
            self.read_bps = Some((self.window_bytes as f64 / seconds).max(0.0));
            self.window_start = now;
            self.window_bytes = 0;
        }
    }

    pub fn update_media_required_window(&mut self, packet_size: usize) {
        self.media_rate_window_bytes = self
            .media_rate_window_bytes
            .saturating_add(packet_size as u64);
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.media_rate_window_start);
        if dt >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
            let seconds = dt.as_secs_f64().max(1e-6);
            self.media_required_bps =
                Some((self.media_rate_window_bytes as f64 / seconds).max(0.0));
            self.media_rate_window_start = now;
            self.media_rate_window_bytes = 0;
        }
    }

    pub fn read_bps(&self) -> Option<f64> {
        self.read_bps
    }

    pub fn media_required_bps(&self) -> Option<f64> {
        self.media_required_bps
    }
}
