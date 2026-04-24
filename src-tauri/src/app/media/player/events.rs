use serde::Serialize;

pub const MEDIA_STATE_EVENT: &str = "media://state";
pub const MEDIA_METADATA_EVENT: &str = "media://metadata";
pub const MEDIA_ERROR_EVENT: &str = "media://error";
pub const MEDIA_DEBUG_EVENT: &str = "media://debug";

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
