use crate::app::media::types::{
    HardwareDecodeMode, PlaybackQualityMode, PlaybackState, PlaybackStatus,
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
        let adaptive_quality_supported = is_adaptive_quality_source(&source);
        self.state.current_path = Some(source);
        self.state.position_seconds = 0.0;
        self.state.duration_seconds = 0.0;
        // 仅“打开”媒体时不应假定已播放，状态应等待真实播放事件驱动。
        self.state.status = PlaybackStatus::Paused;
        self.state.error = None;
        self.state.hw_decode_active = false;
        self.state.hw_decode_backend = None;
        self.state.hw_decode_error = None;
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
        self.state.position_seconds = 0.0;
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

fn is_adaptive_quality_source(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized.contains(".m3u8") || normalized.contains(".mpd")
}
