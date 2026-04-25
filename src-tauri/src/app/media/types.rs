use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareDecodeMode {
    Auto,
    On,
    Off,
}

impl Default for HardwareDecodeMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackQualityMode {
    Source,
    Auto,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "480p")]
    P480,
    #[serde(rename = "320p")]
    P320,
}

impl Default for PlaybackQualityMode {
    fn default() -> Self {
        Self::Source
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub engine: String,
    pub status: PlaybackStatus,
    pub current_path: Option<String>,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub playback_rate: f64,
    pub error: Option<String>,
    pub hw_decode_mode: HardwareDecodeMode,
    pub hw_decode_active: bool,
    pub hw_decode_backend: Option<String>,
    pub hw_decode_error: Option<String>,
    pub quality_mode: PlaybackQualityMode,
    pub adaptive_quality_supported: bool,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            engine: "mpv".to_string(),
            status: PlaybackStatus::Idle,
            current_path: None,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            playback_rate: 1.0,
            error: None,
            hw_decode_mode: HardwareDecodeMode::Auto,
            hw_decode_active: false,
            hw_decode_backend: None,
            hw_decode_error: None,
            quality_mode: PlaybackQualityMode::Source,
            adaptive_quality_supported: false,
        }
    }
}

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
