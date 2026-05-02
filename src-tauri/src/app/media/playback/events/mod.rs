mod payloads;

use serde::Serialize;

pub use payloads::{
    MediaAudioMeterPayload, MediaDecodeQuantileStats, MediaErrorPayload, MediaFrameTypeStats,
    MediaMetadataPayload, MediaTelemetryPayload, MediaVideoStageCostStats,
    MediaVideoTimestampStats,
};

pub const MEDIA_PLAYBACK_STATE_EVENT: &str = "media://playback/state";
pub const MEDIA_PLAYBACK_METADATA_EVENT: &str = "media://playback/metadata";
pub const MEDIA_PLAYBACK_ERROR_EVENT: &str = "media://playback/error";
pub const MEDIA_PLAYBACK_TELEMETRY_EVENT: &str = "media://playback/telemetry";
pub const MEDIA_PLAYBACK_AUDIO_METER_EVENT: &str = "media://playback/audio-meter";
pub const MEDIA_PROTOCOL_VERSION: u32 = 2;

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

pub fn build_media_event<T>(
    event_type: &'static str,
    request_id: Option<String>,
    payload: T,
) -> MediaEventEnvelope<T>
where
    T: Serialize + Clone,
{
    build_media_event_at(event_type, request_id, unix_epoch_ms_now(), payload)
}

pub fn build_media_event_at<T>(
    event_type: &'static str,
    request_id: Option<String>,
    emitted_at_ms: u64,
    payload: T,
) -> MediaEventEnvelope<T>
where
    T: Serialize + Clone,
{
    MediaEventEnvelope {
        protocol_version: MEDIA_PROTOCOL_VERSION,
        event_type,
        request_id,
        emitted_at_ms,
        payload,
    }
}

pub(crate) fn unix_epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
