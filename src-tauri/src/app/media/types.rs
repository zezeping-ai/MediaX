use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Paused,
    Stopped,
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
