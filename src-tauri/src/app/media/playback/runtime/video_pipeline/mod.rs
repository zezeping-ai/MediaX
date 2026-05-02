mod constants;
mod drain;
mod frame_pipeline;
mod frame_stats;
mod process_metrics;
mod stats;
mod telemetry;

use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use std::time::Instant;
use tauri::AppHandle;

pub(crate) use constants::DECODE_LEAD_SLEEP_MS;
pub(crate) use drain::drain_frames;
pub(super) use frame_pipeline::VideoFramePipeline;
pub(crate) use frame_stats::{VideoFrameTypeMetricsRef, VideoTimestampMetricsRef};
pub(super) use process_metrics::ProcessMetricsSampler;
pub(crate) use stats::percentile_from_sorted;

pub(super) struct DrainFramesContext<'a> {
    pub app: &'a AppHandle,
    pub renderer: &'a RendererState,
    pub input_ctx: &'a format::context::Input,
    pub decoder: &'a mut ffmpeg::decoder::Video,
    pub video_time_base: ffmpeg::Rational,
    pub scaler: &'a mut Option<ScalingContext>,
    pub duration_seconds: f64,
    pub output_width: u32,
    pub output_height: u32,
    pub stop_flag: &'a std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub playback_clock: &'a mut PlaybackClock,
    pub last_progress_emit: &'a mut Instant,
    pub current_position_seconds: &'a mut f64,
    pub audio_clock: Option<AudioClock>,
    pub audio_queue_depth_sources: Option<usize>,
    pub active_seek_target_seconds: &'a mut Option<f64>,
    pub last_video_pts_seconds: &'a mut Option<f64>,
    pub fps_window: &'a mut FpsWindow,
    pub frame_pipeline: &'a mut VideoFramePipeline,
    pub process_metrics: &'a mut ProcessMetricsSampler,
    pub audio_allowed_lead_seconds: f64,
    pub network_read_bps: Option<f64>,
    pub media_required_bps: Option<f64>,
    pub is_network_source: bool,
    pub is_realtime_source: bool,
    pub video_timestamp_metrics: VideoTimestampMetricsRef<'a>,
    pub video_frame_type_metrics: VideoFrameTypeMetricsRef<'a>,
    pub video_packet_soft_error_count: &'a mut u64,
    pub stream_generation: u32,
    pub max_frames_per_pass: Option<usize>,
}

impl<'a> DrainFramesContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        app: &'a AppHandle,
        renderer: &'a RendererState,
        input_ctx: &'a format::context::Input,
        decoder: &'a mut ffmpeg::decoder::Video,
        video_time_base: ffmpeg::Rational,
        scaler: &'a mut Option<ScalingContext>,
        duration_seconds: f64,
        output_width: u32,
        output_height: u32,
        stop_flag: &'a std::sync::Arc<std::sync::atomic::AtomicBool>,
        playback_clock: &'a mut PlaybackClock,
        last_progress_emit: &'a mut Instant,
        current_position_seconds: &'a mut f64,
        audio_clock: Option<AudioClock>,
        audio_queue_depth_sources: Option<usize>,
        active_seek_target_seconds: &'a mut Option<f64>,
        last_video_pts_seconds: &'a mut Option<f64>,
        fps_window: &'a mut FpsWindow,
        frame_pipeline: &'a mut VideoFramePipeline,
        process_metrics: &'a mut ProcessMetricsSampler,
        audio_allowed_lead_seconds: f64,
        network_read_bps: Option<f64>,
        media_required_bps: Option<f64>,
        is_network_source: bool,
        is_realtime_source: bool,
        video_timestamp_metrics: VideoTimestampMetricsRef<'a>,
        video_frame_type_metrics: VideoFrameTypeMetricsRef<'a>,
        video_packet_soft_error_count: &'a mut u64,
        stream_generation: u32,
        max_frames_per_pass: Option<usize>,
    ) -> Self {
        Self {
            app,
            renderer,
            input_ctx,
            decoder,
            video_time_base,
            scaler,
            duration_seconds,
            output_width,
            output_height,
            stop_flag,
            playback_clock,
            last_progress_emit,
            current_position_seconds,
            audio_clock,
            audio_queue_depth_sources,
            active_seek_target_seconds,
            last_video_pts_seconds,
            fps_window,
            frame_pipeline,
            process_metrics,
            audio_allowed_lead_seconds,
            network_read_bps,
            media_required_bps,
            is_network_source,
            is_realtime_source,
            video_timestamp_metrics,
            video_frame_type_metrics,
            video_packet_soft_error_count,
            stream_generation,
            max_frames_per_pass,
        }
    }
}
