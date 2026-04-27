use crate::app::media::playback::runtime::emit::emit_debug;
use crate::app::media::playback::runtime::video_pipeline::percentile_from_sorted;
use crate::app::media::playback::runtime::METRICS_EMIT_INTERVAL_MS;
use ffmpeg_next::Packet;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(crate) struct VideoPacketMetrics {
    pub demux_window_start: Instant,
    pub demux_packets: u64,
    pub demux_key_packets: u64,
    pub demux_bytes: u64,
    pub packets_since_last_key: u64,
    pub keyint_ema_packets: f64,
    pub scene_cut_events: u64,
    pub last_video_key_pts_seconds: Option<f64>,
    pub key_intervals_seconds: Vec<f64>,
    pub soft_error_count: u64,
}

impl VideoPacketMetrics {
    pub fn new(now: Instant) -> Self {
        Self {
            demux_window_start: now,
            demux_packets: 0,
            demux_key_packets: 0,
            demux_bytes: 0,
            packets_since_last_key: 0,
            keyint_ema_packets: 0.0,
            scene_cut_events: 0,
            last_video_key_pts_seconds: None,
            key_intervals_seconds: Vec::new(),
            soft_error_count: 0,
        }
    }

    pub fn record_video_packet(
        &mut self,
        app: &AppHandle,
        packet: &Packet,
        video_time_base: ffmpeg_next::Rational,
    ) {
        self.demux_packets = self.demux_packets.saturating_add(1);
        self.packets_since_last_key = self.packets_since_last_key.saturating_add(1);
        self.demux_bytes = self
            .demux_bytes
            .saturating_add(u64::try_from(packet.size()).unwrap_or(0));
        if packet.is_key() {
            self.demux_key_packets = self.demux_key_packets.saturating_add(1);
            if let Some(pts_seconds) = packet
                .pts()
                .map(|pts| (pts as f64) * f64::from(video_time_base))
                .filter(|value| value.is_finite() && *value >= 0.0)
            {
                if let Some(last_key_pts) = self.last_video_key_pts_seconds {
                    let key_interval = (pts_seconds - last_key_pts).max(0.0);
                    if key_interval > 0.0 {
                        self.key_intervals_seconds.push(key_interval);
                    }
                }
                self.last_video_key_pts_seconds = Some(pts_seconds);
            }
            let key_gap_packets = self.packets_since_last_key.max(1);
            if self.keyint_ema_packets > 0.0
                && (key_gap_packets as f64) < self.keyint_ema_packets * 0.6
            {
                self.scene_cut_events = self.scene_cut_events.saturating_add(1);
            }
            self.keyint_ema_packets = if self.keyint_ema_packets <= 0.0 {
                key_gap_packets as f64
            } else {
                self.keyint_ema_packets * 0.85 + (key_gap_packets as f64) * 0.15
            };
            self.packets_since_last_key = 0;
        }
        let window_elapsed = self.demux_window_start.elapsed();
        if window_elapsed < Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
            return;
        }
        let seconds = window_elapsed.as_secs_f64().max(1e-6);
        let packet_rate = (self.demux_packets as f64) / seconds;
        let bitrate_mbps = ((self.demux_bytes as f64) * 8.0) / seconds / 1_000_000.0;
        let key_ratio = if self.demux_packets > 0 {
            (self.demux_key_packets as f64) * 100.0 / (self.demux_packets as f64)
        } else {
            0.0
        };
        let keyint_est = if self.demux_key_packets > 0 {
            (self.demux_packets as f64) / (self.demux_key_packets as f64)
        } else {
            0.0
        };
        emit_debug(
            app,
            "video_demux",
            format!(
                "packet_rate={:.2}pps bitrate≈{:.3}Mbps key_ratio={:.2}% keyint≈{:.1}pkts packets={} bytes={}",
                packet_rate, bitrate_mbps, key_ratio, keyint_est, self.demux_packets, self.demux_bytes
            ),
        );
        emit_debug(app, "video_gop", self.build_gop_debug_message());
        self.demux_window_start = Instant::now();
        self.demux_packets = 0;
        self.demux_key_packets = 0;
        self.demux_bytes = 0;
    }

    fn build_gop_debug_message(&self) -> String {
        if self.key_intervals_seconds.is_empty() {
            return format!(
                "keyint_ema≈{:.2}pkts scene_cut_events={} key_packets={} keyint_s_p50=n/a keyint_s_p95=n/a",
                self.keyint_ema_packets,
                self.scene_cut_events,
                self.demux_key_packets,
            );
        }
        let mut sorted_intervals = self.key_intervals_seconds.clone();
        sorted_intervals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50 = percentile_from_sorted(&sorted_intervals, 50.0);
        let p95 = percentile_from_sorted(&sorted_intervals, 95.0);
        format!(
            "keyint_ema≈{:.2}pkts scene_cut_events={} key_packets={} keyint_s_p50={:.3}s keyint_s_p95={:.3}s samples={}",
            self.keyint_ema_packets,
            self.scene_cut_events,
            self.demux_key_packets,
            p50,
            p95,
            sorted_intervals.len()
        )
    }

    pub fn increment_soft_error_count(&mut self) {
        self.soft_error_count = self.soft_error_count.saturating_add(1);
    }
}
