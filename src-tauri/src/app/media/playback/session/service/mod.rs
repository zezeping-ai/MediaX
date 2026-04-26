mod source_capabilities;
mod state_transitions;

use crate::app::media::model::{
    HardwareDecodeMode, PlaybackQualityMode, PlaybackState, PlaybackStatus,
};

use self::source_capabilities::supports_adaptive_quality;
use self::state_transitions::{
    reset_playback_metrics, reset_runtime_decode_state, reset_source_playback_state,
};

#[derive(Default)]
pub struct MediaPlaybackService {
    state: PlaybackState,
}

impl MediaPlaybackService {
    pub fn state(&mut self) -> PlaybackState {
        self.state.clone()
    }

    pub fn open(&mut self, source: String) -> PlaybackState {
        let adaptive_quality_supported = supports_adaptive_quality(&source);
        self.state.current_path = Some(source);
        reset_playback_metrics(&mut self.state);
        // 仅“打开”媒体时不应假定已播放，状态应等待真实播放事件驱动。
        self.state.status = PlaybackStatus::Paused;
        self.state.error = None;
        reset_runtime_decode_state(&mut self.state);
        // Opening a new source should not inherit previous source's manual downscale setting.
        self.state.quality_mode = PlaybackQualityMode::Source;
        self.state.adaptive_quality_supported = adaptive_quality_supported;
        self.state()
    }

    pub fn play(&mut self) -> PlaybackState {
        self.state.status = PlaybackStatus::Playing;
        self.state.error = None;
        self.state()
    }

    pub fn pause(&mut self) -> PlaybackState {
        self.state.status = PlaybackStatus::Paused;
        self.state()
    }

    pub fn stop(&mut self) -> PlaybackState {
        self.state.status = PlaybackStatus::Stopped;
        reset_source_playback_state(&mut self.state);
        self.state()
    }

    pub fn seek(&mut self, position_seconds: f64) -> PlaybackState {
        self.state.position_seconds = position_seconds.max(0.0);
        self.state()
    }

    pub fn set_rate(&mut self, playback_rate: f64) -> PlaybackState {
        self.state.playback_rate = playback_rate.clamp(0.25, 3.0);
        self.state()
    }

    pub fn hw_decode_mode(&self) -> HardwareDecodeMode {
        self.state.hw_decode_mode
    }

    pub fn set_hw_decode_mode(&mut self, mode: HardwareDecodeMode) -> PlaybackState {
        self.state.hw_decode_mode = mode;
        self.state()
    }

    pub fn update_hw_decode_status(
        &mut self,
        active: bool,
        backend: Option<String>,
        error: Option<String>,
    ) -> PlaybackState {
        self.state.hw_decode_active = active;
        self.state.hw_decode_backend = backend;
        self.state.hw_decode_error = error;
        self.state()
    }

    pub fn quality_mode(&self) -> PlaybackQualityMode {
        self.state.quality_mode
    }

    pub fn set_quality_mode(&mut self, mode: PlaybackQualityMode) -> PlaybackState {
        self.state.quality_mode = mode;
        self.state()
    }

    pub fn adaptive_quality_supported(&self) -> bool {
        self.state.adaptive_quality_supported
    }

    pub fn sync_position(&mut self, position_seconds: f64, duration_seconds: f64) -> PlaybackState {
        self.state.position_seconds = position_seconds.max(0.0);
        self.state.duration_seconds = duration_seconds.max(0.0);
        self.state()
    }
}
