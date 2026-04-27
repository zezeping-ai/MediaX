use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackStatus {
    Idle,
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HardwareDecodeMode {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackQualityMode {
    #[default]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMediaKind {
    #[default]
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackChannelRouting {
    #[default]
    Stereo,
    LeftToBoth,
    RightToBoth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub engine: String,
    pub status: PlaybackStatus,
    pub media_kind: PlaybackMediaKind,
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
    pub volume: f64,
    pub muted: bool,
    pub left_channel_volume: f64,
    pub right_channel_volume: f64,
    pub left_channel_muted: bool,
    pub right_channel_muted: bool,
    pub channel_routing: PlaybackChannelRouting,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            engine: "mpv".to_string(),
            status: PlaybackStatus::Idle,
            media_kind: PlaybackMediaKind::Video,
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
            volume: 1.0,
            muted: false,
            left_channel_volume: 1.0,
            right_channel_volume: 1.0,
            left_channel_muted: false,
            right_channel_muted: false,
            channel_routing: PlaybackChannelRouting::Stereo,
        }
    }
}
