use crate::app::media::playback::dto::{
    HardwareDecodeMode, PlaybackChannelRouting, PlaybackMediaKind, PlaybackQualityMode,
    PlaybackStatus,
};
use crate::app::media::playback::rate::PlaybackRate;

pub(super) struct PlaybackSessionModel {
    pub engine: String,
    pub source: PlaybackSourceState,
    pub transport: PlaybackTransportState,
    pub decode: PlaybackDecodeState,
    pub audio: PlaybackAudioState,
}

pub(super) struct PlaybackSourceState {
    pub current_path: Option<String>,
    pub media_kind: PlaybackMediaKind,
    pub adaptive_quality_supported: bool,
    pub quality_mode: PlaybackQualityMode,
}

pub(super) struct PlaybackTransportState {
    pub status: PlaybackStatus,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub playback_rate: PlaybackRate,
    pub error: Option<String>,
}

pub(super) struct PlaybackDecodeState {
    pub hw_decode_mode: HardwareDecodeMode,
    pub hw_decode_active: bool,
    pub hw_decode_backend: Option<String>,
    pub hw_decode_error: Option<String>,
}

pub(super) struct PlaybackAudioState {
    pub volume: f64,
    pub muted: bool,
    pub left_channel_volume: f64,
    pub right_channel_volume: f64,
    pub left_channel_muted: bool,
    pub right_channel_muted: bool,
    pub channel_routing: PlaybackChannelRouting,
}

impl Default for PlaybackSessionModel {
    fn default() -> Self {
        Self {
            engine: "mpv".to_string(),
            source: PlaybackSourceState::default(),
            transport: PlaybackTransportState::default(),
            decode: PlaybackDecodeState::default(),
            audio: PlaybackAudioState::default(),
        }
    }
}

impl Default for PlaybackSourceState {
    fn default() -> Self {
        Self {
            current_path: None,
            media_kind: PlaybackMediaKind::Video,
            adaptive_quality_supported: false,
            quality_mode: PlaybackQualityMode::Source,
        }
    }
}

impl Default for PlaybackTransportState {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Idle,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            playback_rate: PlaybackRate::default(),
            error: None,
        }
    }
}

impl Default for PlaybackDecodeState {
    fn default() -> Self {
        Self {
            hw_decode_mode: HardwareDecodeMode::Auto,
            hw_decode_active: false,
            hw_decode_backend: None,
            hw_decode_error: None,
        }
    }
}

impl Default for PlaybackAudioState {
    fn default() -> Self {
        Self {
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
