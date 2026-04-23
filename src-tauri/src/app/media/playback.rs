use crate::app::media::types::{PlaybackState, PlaybackStatus};

#[derive(Default)]
pub struct MediaPlaybackService {
    state: PlaybackState,
}

impl MediaPlaybackService {
    pub fn state(&mut self) -> PlaybackState {
        self.state.clone()
    }

    pub fn open(&mut self, source: String) -> PlaybackState {
        self.state.current_path = Some(source);
        self.state.position_seconds = 0.0;
        self.state.duration_seconds = 0.0;
        // 仅“打开”媒体时不应假定已播放，状态应等待真实播放事件驱动。
        self.state.status = PlaybackStatus::Paused;
        self.state.error = None;
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

    pub fn sync_position(&mut self, position_seconds: f64, duration_seconds: f64) -> PlaybackState {
        self.state.position_seconds = position_seconds.max(0.0);
        self.state.duration_seconds = duration_seconds.max(0.0);
        self.state()
    }
}
