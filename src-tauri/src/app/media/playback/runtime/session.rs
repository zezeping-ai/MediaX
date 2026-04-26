use super::audio::clamp_playback_rate;
use super::clock::{AudioClock, FpsWindow, PlaybackClock};
use super::video_pipeline::{percentile_from_sorted, ProcessMetricsSampler, VideoFramePipeline};
use super::{emit_debug, METRICS_EMIT_INTERVAL_MS, RATE_SWITCH_SETTLE_WINDOW_MS};
use crate::app::media::error::MediaError;
use crate::app::media::state::{MediaState, TimingControls};
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::media::Type;
use ffmpeg_next::Dictionary;
use ffmpeg_next::Packet;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

pub(super) struct CacheRemuxWriter {
    pub output_path: String,
    output_ctx: format::context::Output,
    stream_mapping: HashMap<usize, usize>,
}

impl CacheRemuxWriter {
    pub fn new(input_ctx: &format::context::Input, output_path: &str) -> Result<Self, String> {
        let mut output_ctx = format::output(output_path)
            .map_err(|err| format!("open cache output failed: {err}"))?;
        let mut stream_mapping = HashMap::new();
        for stream in input_ctx.streams() {
            let medium = stream.parameters().medium();
            if medium != Type::Video && medium != Type::Audio {
                continue;
            }
            let mut out_stream = output_ctx
                .add_stream(codec::encoder::find(codec::Id::None))
                .map_err(|err| format!("add output stream failed: {err}"))?;
            out_stream.set_parameters(stream.parameters());
            out_stream.set_time_base(stream.time_base());
            stream_mapping.insert(stream.index(), out_stream.index());
        }
        if stream_mapping.is_empty() {
            return Err("cache recording requires at least one audio/video stream".to_string());
        }
        let output_path_lower = output_path.to_ascii_lowercase();
        if output_path_lower.ends_with(".ts") {
            let mut options = Dictionary::new();
            options.set("flush_packets", "1");
            output_ctx
                .write_header_with(options)
                .map_err(|err| format!("write cache header failed: {err}"))?;
        } else {
            let mut options = Dictionary::new();
            options.set("movflags", "frag_keyframe+empty_moov+default_base_moof");
            output_ctx
                .write_header_with(options)
                .map_err(|err| format!("write cache header failed: {err}"))?;
        }
        Ok(Self {
            output_path: output_path.to_string(),
            output_ctx,
            stream_mapping,
        })
    }

    pub fn write_packet(
        &mut self,
        input_ctx: &format::context::Input,
        packet: &Packet,
    ) -> Result<(), String> {
        let Some(&out_stream_index) = self.stream_mapping.get(&packet.stream()) else {
            return Ok(());
        };
        let in_stream = input_ctx
            .stream(packet.stream())
            .ok_or_else(|| "input stream not found".to_string())?;
        let out_stream = self
            .output_ctx
            .stream(out_stream_index)
            .ok_or_else(|| "output stream not found".to_string())?;
        let mut remux_packet = packet.clone();
        remux_packet.set_stream(out_stream_index);
        remux_packet.rescale_ts(in_stream.time_base(), out_stream.time_base());
        remux_packet.set_position(-1);
        remux_packet
            .write_interleaved(&mut self.output_ctx)
            .map_err(|err| format!("write cache packet failed: {err}"))
    }

    pub fn finish(&mut self) {
        let _ = self.output_ctx.write_trailer();
    }
}

pub(super) fn update_cache_session_error(app: &AppHandle, source: &str, message: String) {
    if let Ok(mut guard) = app.state::<MediaState>().cache_recorder.lock() {
        if let Some(session) = guard.as_mut() {
            if session.source == source {
                session.active = false;
                session.error_message = Some(message);
            }
        }
    }
}

pub(super) struct DecodeLoopState {
    pub last_applied_audio_rate: f32,
    pub playback_clock: PlaybackClock,
    pub last_progress_emit: Instant,
    pub current_position_seconds: f64,
    pub audio_clock: Option<AudioClock>,
    pub audio_queue_depth_sources: Option<usize>,
    pub active_seek_target_seconds: Option<f64>,
    pub last_video_pts_seconds: Option<f64>,
    pub rate_switch_settle_until: Option<Instant>,
    pub fps_window: FpsWindow,
    pub frame_pipeline: VideoFramePipeline,
    pub process_metrics: ProcessMetricsSampler,
    pub cache_writer: Option<CacheRemuxWriter>,
    pub net_window_start: Instant,
    pub net_window_bytes: u64,
    pub net_read_bps: Option<f64>,
    pub media_rate_window_start: Instant,
    pub media_rate_window_bytes: u64,
    pub media_required_bps: Option<f64>,
    pub video_demux_window_start: Instant,
    pub video_demux_packets: u64,
    pub video_demux_key_packets: u64,
    pub video_demux_bytes: u64,
    pub video_packets_since_last_key: u64,
    pub video_keyint_ema_packets: f64,
    pub video_scene_cut_events: u64,
    pub last_video_key_pts_seconds: Option<f64>,
    pub video_key_intervals_seconds: Vec<f64>,
    pub video_ts_window_start: Instant,
    pub video_ts_samples: u64,
    pub video_pts_missing: u64,
    pub video_pts_backtrack: u64,
    pub video_pts_jitter_abs_sum_ms: f64,
    pub video_pts_jitter_max_ms: f64,
    pub video_frame_type_window_start: Instant,
    pub video_frame_type_i: u64,
    pub video_frame_type_p: u64,
    pub video_frame_type_b: u64,
    pub video_frame_type_other: u64,
}

impl DecodeLoopState {
    pub fn new(fps_value: f64, timing_controls: Arc<TimingControls>) -> Self {
        let now = Instant::now();
        Self {
            last_applied_audio_rate: clamp_playback_rate(timing_controls.playback_rate()),
            playback_clock: PlaybackClock::new(
                fps_value,
                super::MAX_EMIT_FPS,
                0.0,
                timing_controls,
            ),
            last_progress_emit: Instant::now() - Duration::from_millis(250),
            current_position_seconds: 0.0,
            audio_clock: None,
            audio_queue_depth_sources: None,
            active_seek_target_seconds: None,
            last_video_pts_seconds: None,
            rate_switch_settle_until: None,
            fps_window: FpsWindow::default(),
            frame_pipeline: VideoFramePipeline::default(),
            process_metrics: ProcessMetricsSampler::new(),
            cache_writer: None,
            net_window_start: now,
            net_window_bytes: 0,
            net_read_bps: None,
            media_rate_window_start: now,
            media_rate_window_bytes: 0,
            media_required_bps: None,
            video_demux_window_start: now,
            video_demux_packets: 0,
            video_demux_key_packets: 0,
            video_demux_bytes: 0,
            video_packets_since_last_key: 0,
            video_keyint_ema_packets: 0.0,
            video_scene_cut_events: 0,
            last_video_key_pts_seconds: None,
            video_key_intervals_seconds: Vec::new(),
            video_ts_window_start: now,
            video_ts_samples: 0,
            video_pts_missing: 0,
            video_pts_backtrack: 0,
            video_pts_jitter_abs_sum_ms: 0.0,
            video_pts_jitter_max_ms: 0.0,
            video_frame_type_window_start: now,
            video_frame_type_i: 0,
            video_frame_type_p: 0,
            video_frame_type_b: 0,
            video_frame_type_other: 0,
        }
    }

    pub fn update_network_window(&mut self, packet_size: usize) {
        self.net_window_bytes = self.net_window_bytes.saturating_add(packet_size as u64);
        let now = Instant::now();
        let dt = now.saturating_duration_since(self.net_window_start);
        if dt >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
            let seconds = dt.as_secs_f64().max(1e-6);
            self.net_read_bps = Some((self.net_window_bytes as f64 / seconds).max(0.0));
            self.net_window_start = now;
            self.net_window_bytes = 0;
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

    pub fn in_rate_switch_settle(&self) -> bool {
        self.rate_switch_settle_until
            .map(|deadline| Instant::now() < deadline)
            .unwrap_or(false)
    }

    pub fn begin_rate_switch_settle(&mut self) {
        self.rate_switch_settle_until =
            Some(Instant::now() + Duration::from_millis(RATE_SWITCH_SETTLE_WINDOW_MS));
    }

    pub fn record_video_packet(
        &mut self,
        app: &AppHandle,
        packet: &Packet,
        video_time_base: ffmpeg_next::Rational,
    ) {
        self.video_demux_packets = self.video_demux_packets.saturating_add(1);
        self.video_packets_since_last_key = self.video_packets_since_last_key.saturating_add(1);
        self.video_demux_bytes = self
            .video_demux_bytes
            .saturating_add(u64::try_from(packet.size()).unwrap_or(0));
        if packet.is_key() {
            self.video_demux_key_packets = self.video_demux_key_packets.saturating_add(1);
            if let Some(pts_seconds) = packet
                .pts()
                .map(|pts| (pts as f64) * f64::from(video_time_base))
                .filter(|v| v.is_finite() && *v >= 0.0)
            {
                if let Some(last_key_pts) = self.last_video_key_pts_seconds {
                    let key_interval = (pts_seconds - last_key_pts).max(0.0);
                    if key_interval > 0.0 {
                        self.video_key_intervals_seconds.push(key_interval);
                    }
                }
                self.last_video_key_pts_seconds = Some(pts_seconds);
            }
            let key_gap_packets = self.video_packets_since_last_key.max(1);
            if self.video_keyint_ema_packets > 0.0
                && (key_gap_packets as f64) < self.video_keyint_ema_packets * 0.6
            {
                self.video_scene_cut_events = self.video_scene_cut_events.saturating_add(1);
            }
            self.video_keyint_ema_packets = if self.video_keyint_ema_packets <= 0.0 {
                key_gap_packets as f64
            } else {
                self.video_keyint_ema_packets * 0.85 + (key_gap_packets as f64) * 0.15
            };
            self.video_packets_since_last_key = 0;
        }
        let window_elapsed = self.video_demux_window_start.elapsed();
        if window_elapsed >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
            let seconds = window_elapsed.as_secs_f64().max(1e-6);
            let packet_rate = (self.video_demux_packets as f64) / seconds;
            let bitrate_mbps = ((self.video_demux_bytes as f64) * 8.0) / seconds / 1_000_000.0;
            let key_ratio = if self.video_demux_packets > 0 {
                (self.video_demux_key_packets as f64) * 100.0 / (self.video_demux_packets as f64)
            } else {
                0.0
            };
            let keyint_est = if self.video_demux_key_packets > 0 {
                (self.video_demux_packets as f64) / (self.video_demux_key_packets as f64)
            } else {
                0.0
            };
            emit_debug(
                app,
                "video_demux",
                format!(
                    "packet_rate={:.2}pps bitrate≈{:.3}Mbps key_ratio={:.2}% keyint≈{:.1}pkts packets={} bytes={}",
                    packet_rate, bitrate_mbps, key_ratio, keyint_est, self.video_demux_packets, self.video_demux_bytes
                ),
            );
            emit_debug(
                app,
                "video_gop",
                if self.video_key_intervals_seconds.is_empty() {
                    format!(
                        "keyint_ema≈{:.2}pkts scene_cut_events={} key_packets={} keyint_s_p50=n/a keyint_s_p95=n/a",
                        self.video_keyint_ema_packets, self.video_scene_cut_events, self.video_demux_key_packets
                    )
                } else {
                    let mut sorted_intervals = self.video_key_intervals_seconds.clone();
                    sorted_intervals
                        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let p50 = percentile_from_sorted(&sorted_intervals, 50.0);
                    let p95 = percentile_from_sorted(&sorted_intervals, 95.0);
                    format!(
                        "keyint_ema≈{:.2}pkts scene_cut_events={} key_packets={} keyint_s_p50={:.3}s keyint_s_p95={:.3}s samples={}",
                        self.video_keyint_ema_packets,
                        self.video_scene_cut_events,
                        self.video_demux_key_packets,
                        p50,
                        p95,
                        sorted_intervals.len()
                    )
                },
            );
            self.video_demux_window_start = Instant::now();
            self.video_demux_packets = 0;
            self.video_demux_key_packets = 0;
            self.video_demux_bytes = 0;
        }
    }
}

pub(super) fn current_recording_target(
    app: &AppHandle,
    source: &str,
) -> Result<Option<String>, String> {
    let media_state = app.state::<MediaState>();
    let guard = media_state
        .cache_recorder
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("cache recorder").to_string())?;
    Ok(guard.as_ref().and_then(|session| {
        (session.active && session.source == source).then(|| session.output_path.clone())
    }))
}
