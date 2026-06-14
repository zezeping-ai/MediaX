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
pub struct LyricsCandidateSummary {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub synced: bool,
    pub preview: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsSearchHit {
    pub id: String,
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_seconds: Option<f64>,
    pub synced: bool,
    pub preview: String,
    pub lyrics_lrc: String,
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
