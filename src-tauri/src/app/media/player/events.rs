use serde::Serialize;

pub const MEDIA_STATE_EVENT: &str = "media://state";
pub const MEDIA_STATE_EVENT_V2: &str = "media://state/v2";
pub const MEDIA_METADATA_EVENT: &str = "media://metadata";
pub const MEDIA_ERROR_EVENT: &str = "media://error";
pub const MEDIA_DEBUG_EVENT: &str = "media://debug";
pub const MEDIA_DEBUG_EVENT_V2: &str = "media://debug/v2";
pub const MEDIA_TELEMETRY_EVENT_V2: &str = "media://telemetry/v2";
pub const MEDIA_PLAYBACK_STATE_EVENT: &str = "media://playback/state";
pub const MEDIA_PLAYBACK_METADATA_EVENT: &str = "media://playback/metadata";
pub const MEDIA_PLAYBACK_ERROR_EVENT: &str = "media://playback/error";
pub const MEDIA_PLAYBACK_DEBUG_EVENT: &str = "media://playback/debug";
pub const MEDIA_PLAYBACK_TELEMETRY_EVENT: &str = "media://playback/telemetry";
pub const MEDIA_PROTOCOL_VERSION: u32 = 2;

#[derive(Clone, Serialize)]
pub struct MediaMetadataPayload {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
}

#[derive(Clone, Serialize)]
pub struct MediaErrorPayload {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct MediaDebugPayload {
    pub stage: &'static str,
    pub message: String,
    pub at_ms: u64,
}

#[derive(Clone, Serialize)]
pub struct MediaEventEnvelope<T>
where
    T: Serialize + Clone,
{
    pub protocol_version: u32,
    pub event_type: &'static str,
    pub request_id: Option<String>,
    pub emitted_at_ms: u64,
    pub payload: T,
}

#[derive(Clone, Serialize)]
pub struct MediaTelemetryPayload {
    pub source_fps: f64,
    pub render_fps: f64,
    pub queue_depth: usize,
    pub clock_seconds: f64,
    pub network_read_bytes_per_second: Option<f64>,
    pub audio_drift_seconds: Option<f64>,
    pub video_pts_gap_seconds: Option<f64>,
    pub seek_settle_ms: Option<u64>,
    pub decode_avg_frame_cost_ms: Option<f64>,
    pub decode_max_frame_cost_ms: Option<f64>,
    pub decode_samples: Option<u64>,
    pub process_cpu_percent: Option<f32>,
    pub process_memory_mb: Option<f64>,
    pub gpu_queue_depth: Option<usize>,
    pub gpu_queue_capacity: Option<usize>,
    pub gpu_queue_utilization: Option<f64>,
    pub render_estimated_cost_ms: Option<f64>,
    pub render_present_lag_ms: Option<f64>,
}
