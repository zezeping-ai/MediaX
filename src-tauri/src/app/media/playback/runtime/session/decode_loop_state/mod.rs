mod network_metrics;
mod video_frame_metrics;
mod video_packet_metrics;

use super::cache_remux::CacheRemuxWriter;
use crate::app::media::playback::rate::{PlaybackRate, RATE_SWITCH_SETTLE_WINDOW_MS};
use crate::app::media::playback::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
use crate::app::media::playback::runtime::sync_clock::SyncClockSample;
use crate::app::media::playback::runtime::video_pipeline::{
    ProcessMetricsSampler, VideoFramePipeline,
};
use crate::app::media::playback::runtime::MAX_EMIT_FPS;
use crate::app::media::state::TimingControls;
use ffmpeg_next::Packet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(crate) use network_metrics::NetworkMetrics;
pub(crate) use video_frame_metrics::{VideoFrameTypeMetrics, VideoTimestampMetrics};
pub(crate) use video_packet_metrics::VideoPacketMetrics;

const AUDIO_SYNC_WARMUP_WINDOW_SECONDS: f64 = 2.5;

pub(crate) struct DecodeLoopState {
    pub last_applied_audio_rate: PlaybackRate,
    pub pending_audio_rate: Option<PlaybackRate>,
    pub playback_clock: PlaybackClock,
    pub last_progress_emit: Instant,
    // Transport progress for UI/progress emission. This is allowed to be slightly predictive
    // and must not be treated as the authoritative rendered video head.
    pub progress_position_seconds: f64,
    // Queue-derived scheduling clock used by decode, pacing, and sync decisions.
    pub audio_clock: Option<AudioClock>,
    // Backend-observed playback head used for telemetry and future bounded correction only.
    pub observed_audio_clock: Option<SyncClockSample>,
    pub audio_queue_depth_sources: Option<usize>,
    pub audio_queued_seconds: Option<f64>,
    pub pause_prefetch_mode: bool,
    pub pause_prefetch_logged_buffered_seconds: Option<f64>,
    pub active_seek_target_seconds: Option<f64>,
    pub seek_refill_until: Option<Instant>,
    pub seek_settle_until: Option<Instant>,
    pub audio_sync_warmup_until: Option<Instant>,
    pub video_queue_boost_until: Option<Instant>,
    pub last_video_pts_seconds: Option<f64>,
    pub rate_switch_settle_until: Option<Instant>,
    pub rate_switch_hold_logged: bool,
    pub fps_window: FpsWindow,
    pub frame_pipeline: VideoFramePipeline,
    pub process_metrics: ProcessMetricsSampler,
    pub cache_writer: Option<CacheRemuxWriter>,
    pub network_metrics: NetworkMetrics,
    pub video_packet_metrics: VideoPacketMetrics,
    pub video_timestamp_metrics: VideoTimestampMetrics,
    pub video_frame_type_metrics: VideoFrameTypeMetrics,
}

impl DecodeLoopState {
    pub fn new(
        fps_value: f64,
        timing_controls: Arc<TimingControls>,
        is_realtime_source: bool,
    ) -> Self {
        let now = Instant::now();
        Self {
            last_applied_audio_rate:
                crate::app::media::playback::runtime::audio::effective_playback_rate(
                    timing_controls.playback_rate_value(),
                    is_realtime_source,
                ),
            pending_audio_rate: None,
            playback_clock: PlaybackClock::new(
                fps_value,
                MAX_EMIT_FPS,
                0.0,
                timing_controls,
                is_realtime_source,
            ),
            last_progress_emit: Instant::now() - Duration::from_millis(250),
            progress_position_seconds: 0.0,
            audio_clock: None,
            observed_audio_clock: None,
            audio_queue_depth_sources: None,
            audio_queued_seconds: None,
            pause_prefetch_mode: false,
            pause_prefetch_logged_buffered_seconds: None,
            active_seek_target_seconds: None,
            seek_refill_until: None,
            seek_settle_until: None,
            audio_sync_warmup_until: None,
            video_queue_boost_until: None,
            last_video_pts_seconds: None,
            rate_switch_settle_until: None,
            rate_switch_hold_logged: false,
            fps_window: FpsWindow::default(),
            frame_pipeline: VideoFramePipeline::default(),
            process_metrics: ProcessMetricsSampler::new(),
            cache_writer: None,
            network_metrics: NetworkMetrics::new(now),
            video_packet_metrics: VideoPacketMetrics::new(now),
            video_timestamp_metrics: VideoTimestampMetrics::new(now),
            video_frame_type_metrics: VideoFrameTypeMetrics::new(now),
        }
    }

    pub fn update_network_window(&mut self, packet_size: usize) {
        self.network_metrics.update_network_window(packet_size);
    }

    pub fn update_media_required_window(&mut self, packet_size: usize) {
        self.network_metrics
            .update_media_required_window(packet_size);
    }

    pub fn network_read_bps(&self) -> Option<f64> {
        self.network_metrics.read_bps()
    }

    pub fn media_required_bps(&self) -> Option<f64> {
        self.network_metrics.media_required_bps()
    }

    pub fn record_video_packet(
        &mut self,
        app: &AppHandle,
        packet: &Packet,
        video_time_base: ffmpeg_next::Rational,
    ) {
        self.video_packet_metrics
            .record_video_packet(app, packet, video_time_base);
    }

    pub fn increment_video_soft_error_count(&mut self) {
        self.video_packet_metrics.increment_soft_error_count();
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

    pub fn reset_audio_sync_state(&mut self) {
        self.audio_clock = None;
        self.observed_audio_clock = None;
        self.audio_queue_depth_sources = None;
        self.audio_queued_seconds = None;
    }

    pub fn audio_sync_warmup_factor(&self) -> f64 {
        let Some(deadline) = self.audio_sync_warmup_until else {
            return 0.0;
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        let remaining_seconds = remaining.as_secs_f64();
        if remaining_seconds <= 0.0 {
            return 0.0;
        }
        (remaining_seconds / AUDIO_SYNC_WARMUP_WINDOW_SECONDS).clamp(0.0, 1.0)
    }

    pub fn in_seek_refill(&self) -> bool {
        self.seek_refill_until
            .map(|deadline| Instant::now() < deadline)
            .unwrap_or(false)
    }

    pub fn begin_seek_refill(&mut self, duration: Duration) {
        self.seek_refill_until = Some(Instant::now() + duration);
    }

    pub fn clear_seek_refill(&mut self) {
        self.seek_refill_until = None;
    }

    pub fn in_seek_settle(&self) -> bool {
        self.seek_settle_until
            .map(|deadline| Instant::now() < deadline)
            .unwrap_or(false)
    }

    pub fn begin_seek_settle(&mut self, duration: Duration) {
        self.seek_settle_until = Some(Instant::now() + duration);
    }

    pub fn begin_audio_sync_warmup(&mut self, duration: Duration) {
        self.audio_sync_warmup_until = Some(Instant::now() + duration);
    }

    pub fn in_video_queue_boost(&self) -> bool {
        self.video_queue_boost_until
            .map(|deadline| Instant::now() < deadline)
            .unwrap_or(false)
    }

    pub fn begin_video_queue_boost(&mut self, duration: Duration) {
        self.video_queue_boost_until = Some(Instant::now() + duration);
    }

    pub fn commit_audio_playback_rate(&mut self, playback_rate: PlaybackRate) {
        self.last_applied_audio_rate = playback_rate;
        self.pending_audio_rate = None;
        self.rate_switch_hold_logged = false;
    }

    pub fn schedule_audio_rate_switch(&mut self, playback_rate: PlaybackRate) {
        self.pending_audio_rate = Some(playback_rate);
        self.rate_switch_hold_logged = false;
    }

    pub fn clear_pending_audio_rate_switch(&mut self) {
        self.pending_audio_rate = None;
        self.rate_switch_hold_logged = false;
    }
}
