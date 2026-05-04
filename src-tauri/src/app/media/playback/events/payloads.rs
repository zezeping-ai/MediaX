use serde::Serialize;

use crate::app::media::model::MediaLyricLine;
use crate::app::media::playback::dto::PlaybackMediaKind;

#[derive(Clone, Serialize)]
pub struct MediaMetadataPayload {
    pub media_kind: PlaybackMediaKind,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub has_cover_art: bool,
    pub lyrics: Vec<MediaLyricLine>,
}

#[derive(Clone, Serialize)]
pub struct MediaErrorPayload {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct MediaAudioMeterPayload {
    pub sample_rate: u32,
    pub channels: u16,
    pub left_peak: f32,
    pub right_peak: f32,
    pub left_spectrum: Vec<f32>,
    pub right_spectrum: Vec<f32>,
}

#[derive(Clone, Serialize)]
pub struct MediaVideoTimestampStats {
    pub samples: u64,
    pub pts_missing_ratio_percent: f64,
    pub pts_backtrack_count: u64,
    pub jitter_avg_ms: f64,
    pub jitter_max_ms: f64,
}

#[derive(Clone, Serialize)]
pub struct MediaFrameTypeStats {
    pub sample_count: u64,
    pub i_ratio_percent: f64,
    pub p_ratio_percent: f64,
    pub b_ratio_percent: f64,
    pub other_ratio_percent: f64,
}

#[derive(Clone, Serialize)]
pub struct MediaDecodeQuantileStats {
    pub sample_count: u64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

#[derive(Clone, Serialize)]
pub struct MediaVideoStageCostStats {
    pub sample_count: u64,
    pub receive_avg_ms: f64,
    pub receive_max_ms: f64,
    pub queue_wait_avg_ms: f64,
    pub queue_wait_max_ms: f64,
    pub hw_transfer_avg_ms: f64,
    pub hw_transfer_max_ms: f64,
    pub scale_avg_ms: f64,
    pub scale_max_ms: f64,
    pub color_profile_avg_ms: f64,
    pub color_profile_max_ms: f64,
    pub frame_extract_avg_ms: f64,
    pub frame_extract_max_ms: f64,
    pub upload_prep_avg_ms: f64,
    pub upload_prep_max_ms: f64,
    pub submit_avg_ms: f64,
    pub submit_max_ms: f64,
    pub total_avg_ms: f64,
    pub total_max_ms: f64,
}

#[derive(Clone, Serialize)]
pub struct MediaTelemetryPayload {
    pub source_fps: f64,
    pub render_fps: f64,
    pub queue_depth: usize,
    pub audio_queue_depth_sources: Option<usize>,
    pub progress_clock_seconds: f64,
    pub display_video_pts_seconds: Option<f64>,
    pub effective_display_video_pts_seconds: Option<f64>,
    pub sync_video_pts_seconds: Option<f64>,
    pub presented_video_pts_seconds: Option<f64>,
    pub submitted_video_pts_seconds: Option<f64>,
    pub current_audio_clock_seconds: Option<f64>,
    pub current_frame_type: Option<String>,
    pub current_frame_width: Option<u32>,
    pub current_frame_height: Option<u32>,
    pub playback_rate: Option<f64>,
    pub requested_playback_rate: Option<f64>,
    pub effective_playback_rate: Option<f64>,
    pub playback_rate_limited_reason: Option<&'static str>,
    pub network_read_bytes_per_second: Option<f64>,
    pub media_required_bytes_per_second: Option<f64>,
    pub network_sustain_ratio: Option<f64>,
    pub sync_video_minus_audio_seconds: Option<f64>,
    pub video_pts_gap_seconds: Option<f64>,
    pub seek_settle_ms: Option<u64>,
    pub decode_avg_frame_cost_ms: Option<f64>,
    pub decode_max_frame_cost_ms: Option<f64>,
    pub decode_samples: Option<u64>,
    pub decode_quantiles: Option<MediaDecodeQuantileStats>,
    pub video_stage_costs: Option<MediaVideoStageCostStats>,
    pub video_timestamps: Option<MediaVideoTimestampStats>,
    pub frame_types: Option<MediaFrameTypeStats>,
    pub process_cpu_percent: Option<f32>,
    pub process_memory_mb: Option<f64>,
    pub gpu_queue_depth: Option<usize>,
    pub gpu_queue_capacity: Option<usize>,
    pub gpu_queue_utilization: Option<f64>,
    pub render_estimated_cost_ms: Option<f64>,
    pub render_present_lag_ms: Option<f64>,
    pub render_loop_wakeups: Option<u64>,
    pub render_attempts: Option<u64>,
    pub render_presents: Option<u64>,
    pub render_uploads: Option<u64>,
    pub video_submit_lead_ms: Option<f64>,
    pub video_packet_soft_error_count: Option<u64>,
    pub video_frame_drop_count: Option<u64>,
    pub video_hw_transfer_drop_count: Option<u64>,
    pub video_nv12_drop_count: Option<u64>,
    pub video_scale_drop_count: Option<u64>,
}
