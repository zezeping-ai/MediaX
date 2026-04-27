use serde::{Deserialize, Serialize};

// Compatibility re-exports keep the old model entrypoint stable while playback DTOs
// move into the dedicated playback boundary.
#[allow(unused_imports)]
pub use crate::app::media::playback::dto::{
    HardwareDecodeMode, PlaybackChannelRouting, PlaybackMediaKind, PlaybackQualityMode,
    PlaybackState, PlaybackStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub last_played_at: Option<u64>,
    pub last_position_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaLibraryState {
    pub roots: Vec<String>,
    pub items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSnapshot {
    pub playback: PlaybackState,
    pub library: MediaLibraryState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewFrame {
    pub mime_type: String,
    pub data_base64: String,
    pub width: u32,
    pub height: u32,
    pub position_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLyricLine {
    pub time_seconds: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRecordingStatus {
    pub recording: bool,
    pub source: Option<String>,
    pub output_path: Option<String>,
    pub finalized_output_path: Option<String>,
    pub output_size_bytes: Option<u64>,
    pub started_at_ms: Option<u64>,
    pub error_message: Option<String>,
    pub fallback_transcoding: Option<bool>,
}
