mod model;
mod snapshot;
mod source_capabilities;
mod state_transitions;

use crate::app::media::model::{MediaLibraryState, MediaSnapshot};
use crate::app::media::playback::dto::{
    HardwareDecodeMode, PlaybackChannelRouting, PlaybackMediaKind, PlaybackQualityMode,
    PlaybackState, PlaybackStatus,
};
use crate::app::media::playback::rate::PlaybackRate;

use self::model::PlaybackSessionModel;
use self::snapshot::{export_media_snapshot, export_playback_state};
use self::source_capabilities::supports_adaptive_quality;
pub(super) use self::source_capabilities::supports_timeline_seek;
use self::state_transitions::{
    reset_playback_metrics, reset_runtime_decode_state, reset_source_playback_state,
};

#[derive(Default)]
pub struct MediaPlaybackService {
    session: PlaybackSessionModel,
}

impl MediaPlaybackService {
    pub fn state(&self) -> PlaybackState {
        export_playback_state(&self.session)
    }

    pub fn snapshot(&self, library: MediaLibraryState) -> MediaSnapshot {
        export_media_snapshot(&self.session, library)
    }

    pub fn open(&mut self, source: String) -> PlaybackState {
        let adaptive_quality_supported = supports_adaptive_quality(&source);
        self.session.source.current_path = Some(source);
        self.session.source.media_kind = PlaybackMediaKind::Video;
        reset_playback_metrics(&mut self.session);
        // 仅“打开”媒体时不应假定已播放，状态应等待真实播放事件驱动。
        self.session.transport.status = PlaybackStatus::Paused;
        self.session.transport.error = None;
        reset_runtime_decode_state(&mut self.session);
        // Default new playback to the adaptive path so large sources start from the
        // smoother, cross-platform quality profile instead of always forcing source size.
        self.session.source.quality_mode = PlaybackQualityMode::Auto;
        self.session.source.adaptive_quality_supported = adaptive_quality_supported;
        self.state()
    }

    pub fn play(&mut self) -> PlaybackState {
        self.session.transport.status = PlaybackStatus::Playing;
        self.session.transport.error = None;
        self.state()
    }

    pub fn pause(&mut self) -> PlaybackState {
        self.session.transport.status = PlaybackStatus::Paused;
        self.state()
    }

    pub fn stop(&mut self) -> PlaybackState {
        self.session.transport.status = PlaybackStatus::Stopped;
        reset_source_playback_state(&mut self.session);
        self.state()
    }

    pub fn seek(&mut self, position_seconds: f64) -> PlaybackState {
        self.session.transport.position_seconds = position_seconds.max(0.0);
        self.state()
    }

    pub fn set_rate(&mut self, playback_rate: f64) -> PlaybackState {
        self.session.transport.playback_rate = PlaybackRate::from_f64(playback_rate);
        self.state()
    }

    pub fn hw_decode_mode(&self) -> HardwareDecodeMode {
        self.session.decode.hw_decode_mode
    }

    pub fn set_hw_decode_mode(&mut self, mode: HardwareDecodeMode) -> PlaybackState {
        self.session.decode.hw_decode_mode = mode;
        self.state()
    }

    pub fn update_hw_decode_status(
        &mut self,
        active: bool,
        backend: Option<String>,
        error: Option<String>,
    ) -> PlaybackState {
        self.session.decode.hw_decode_active = active;
        self.session.decode.hw_decode_backend = backend;
        self.session.decode.hw_decode_error = error;
        self.state()
    }

    pub fn quality_mode(&self) -> PlaybackQualityMode {
        self.session.source.quality_mode
    }

    pub fn set_quality_mode(&mut self, mode: PlaybackQualityMode) -> PlaybackState {
        self.session.source.quality_mode = mode;
        self.state()
    }

    pub fn set_media_kind(&mut self, kind: PlaybackMediaKind) -> PlaybackState {
        self.session.source.media_kind = kind;
        self.state()
    }

    pub fn set_volume(&mut self, volume: f64) -> PlaybackState {
        self.session.audio.volume = volume.clamp(0.0, 1.0);
        self.session.audio.muted = self.session.audio.volume <= 0.0;
        self.state()
    }

    pub fn set_muted(&mut self, muted: bool) -> PlaybackState {
        self.session.audio.muted = muted;
        self.state()
    }

    pub fn set_left_channel_volume(&mut self, volume: f64) -> PlaybackState {
        self.session.audio.left_channel_volume = volume.clamp(0.0, 1.0);
        self.session.audio.left_channel_muted = self.session.audio.left_channel_volume <= 0.0;
        self.state()
    }

    pub fn set_right_channel_volume(&mut self, volume: f64) -> PlaybackState {
        self.session.audio.right_channel_volume = volume.clamp(0.0, 1.0);
        self.session.audio.right_channel_muted = self.session.audio.right_channel_volume <= 0.0;
        self.state()
    }

    pub fn set_left_channel_muted(&mut self, muted: bool) -> PlaybackState {
        self.session.audio.left_channel_muted = muted;
        self.state()
    }

    pub fn set_right_channel_muted(&mut self, muted: bool) -> PlaybackState {
        self.session.audio.right_channel_muted = muted;
        self.state()
    }

    pub fn set_channel_routing(&mut self, routing: PlaybackChannelRouting) -> PlaybackState {
        self.session.audio.channel_routing = routing;
        self.state()
    }

    pub fn adaptive_quality_supported(&self) -> bool {
        self.session.source.adaptive_quality_supported
    }

    pub fn sync_position(
        &mut self,
        position_seconds: f64,
        duration_seconds: f64,
        buffered_position_seconds: f64,
    ) -> PlaybackState {
        self.session.transport.position_seconds = position_seconds.max(0.0);
        self.session.transport.duration_seconds = duration_seconds.max(0.0);
        self.session.transport.buffered_position_seconds = buffered_position_seconds.max(0.0);
        self.state()
    }
}
