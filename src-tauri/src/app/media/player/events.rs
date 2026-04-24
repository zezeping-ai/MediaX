use serde::Serialize;

pub const MEDIA_STATE_EVENT: &str = "media://state";
pub const MEDIA_STATE_EVENT_V2: &str = "media://state/v2";
pub const MEDIA_METADATA_EVENT: &str = "media://metadata";
pub const MEDIA_ERROR_EVENT: &str = "media://error";
pub const MEDIA_DEBUG_EVENT: &str = "media://debug";
pub const MEDIA_DEBUG_EVENT_V2: &str = "media://debug/v2";
pub const MEDIA_TELEMETRY_EVENT_V2: &str = "media://telemetry/v2";
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
    pub audio_drift_seconds: Option<f64>,
    pub video_pts_gap_seconds: Option<f64>,
    pub seek_settle_ms: Option<u64>,
}
