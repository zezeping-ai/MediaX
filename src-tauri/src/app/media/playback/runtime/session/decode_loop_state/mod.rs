mod network_metrics;
mod video_frame_metrics;
mod video_packet_metrics;

use super::cache_remux::CacheRemuxWriter;
use crate::app::media::playback::rate::{PlaybackRate, RATE_SWITCH_SETTLE_WINDOW_MS};
use crate::app::media::playback::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
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

pub(crate) struct DecodeLoopState {
    pub last_applied_audio_rate: PlaybackRate,
    pub pending_audio_rate: Option<PlaybackRate>,
    pub playback_clock: PlaybackClock,
    pub last_progress_emit: Instant,
    pub current_position_seconds: f64,
    pub audio_clock: Option<AudioClock>,
    pub audio_queue_depth_sources: Option<usize>,
    pub active_seek_target_seconds: Option<f64>,
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
    pub fn new(fps_value: f64, timing_controls: Arc<TimingControls>) -> Self {
        let now = Instant::now();
        Self {
            last_applied_audio_rate: timing_controls.playback_rate_value(),
            pending_audio_rate: None,
            playback_clock: PlaybackClock::new(fps_value, MAX_EMIT_FPS, 0.0, timing_controls),
            last_progress_emit: Instant::now() - Duration::from_millis(250),
            current_position_seconds: 0.0,
            audio_clock: None,
            audio_queue_depth_sources: None,
            active_seek_target_seconds: None,
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
        self.audio_queue_depth_sources = None;
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
