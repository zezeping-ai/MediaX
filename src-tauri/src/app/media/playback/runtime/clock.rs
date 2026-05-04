use crate::app::media::playback::runtime::audio::{
    effective_playback_rate, playback_rate_limited_reason,
};
use crate::app::media::playback::runtime::sync_clock::PlaybackClockInput;
use crate::app::media::state::TimingControls;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct AudioClock {
    anchor_instant: Instant,
    anchor_media_seconds: f64,
    anchor_rate: f64,
}

impl AudioClock {
    pub fn new(anchor_media_seconds: f64, anchor_rate: f64) -> Self {
        Self {
            anchor_instant: Instant::now(),
            anchor_media_seconds: anchor_media_seconds.max(0.0),
            anchor_rate: anchor_rate.max(0.25),
        }
    }

    pub fn now_seconds(&self) -> f64 {
        let elapsed = Instant::now()
            .saturating_duration_since(self.anchor_instant)
            .as_secs_f64();
        (self.anchor_media_seconds + elapsed * self.anchor_rate.max(0.25)).max(0.0)
    }

    pub fn rebase_rate(&mut self, next_rate: f64) {
        let now_media_seconds = self.now_seconds();
        self.anchor_instant = Instant::now();
        self.anchor_media_seconds = now_media_seconds.max(0.0);
        self.anchor_rate = next_rate.max(0.25);
    }

    pub fn rebase_position(&mut self, media_seconds: f64, next_rate: f64) {
        self.anchor_instant = Instant::now();
        self.anchor_media_seconds = media_seconds.max(0.0);
        self.anchor_rate = next_rate.max(0.25);
    }
}

pub struct PlaybackClock {
    frame_duration: Duration,
    last_emit_instant: Option<Instant>,
    media_seconds: f64,
    timing_controls: Arc<TimingControls>,
    is_realtime_source: bool,
}

#[derive(Default)]
pub struct FpsWindow {
    started_at: Option<Instant>,
    frames: u32,
}

impl FpsWindow {
    pub fn record_frame_and_compute(&mut self) -> Option<f64> {
        let now = Instant::now();
        let started_at = self.started_at.get_or_insert(now);
        self.frames = self.frames.saturating_add(1);
        let elapsed = now.saturating_duration_since(*started_at);
        if elapsed >= Duration::from_secs(1) {
            let fps = (self.frames as f64) / elapsed.as_secs_f64().max(1e-6);
            self.started_at = Some(now);
            self.frames = 0;
            return Some(fps);
        }
        None
    }
}

impl PlaybackClock {
    pub fn new(
        fps: f64,
        max_emit_fps: u32,
        start_seconds: f64,
        timing_controls: Arc<TimingControls>,
        is_realtime_source: bool,
    ) -> Self {
        let safe_fps = if fps.is_finite() && fps >= 1.0 {
            fps
        } else {
            30.0
        };
        let limited_fps = if max_emit_fps > 0 {
            safe_fps.min(max_emit_fps as f64)
        } else {
            safe_fps
        };
        Self {
            frame_duration: Duration::from_secs_f64(1.0 / limited_fps.max(1.0)),
            last_emit_instant: None,
            media_seconds: start_seconds.max(0.0),
            timing_controls,
            is_realtime_source,
        }
    }

    pub fn reset_to(&mut self, media_seconds: f64) {
        self.media_seconds = media_seconds.max(0.0);
        self.last_emit_instant = None;
    }

    pub fn frame_duration_seconds(&self) -> f64 {
        self.frame_duration.as_secs_f64()
    }

    pub fn source_fps(&self) -> f64 {
        1.0 / self.frame_duration.as_secs_f64().max(1e-6)
    }

    pub fn requested_playback_rate(&self) -> f64 {
        self.timing_controls.playback_rate_value().as_f64()
    }

    pub fn playback_rate(&self) -> f64 {
        effective_playback_rate(
            self.timing_controls.playback_rate_value(),
            self.is_realtime_source,
        )
        .as_f64()
    }

    pub fn playback_rate_limited_reason(&self) -> Option<&'static str> {
        playback_rate_limited_reason(
            self.timing_controls.playback_rate_value(),
            self.is_realtime_source,
        )
    }

    pub fn tick(&mut self, input: PlaybackClockInput) -> f64 {
        let rate = self.playback_rate().max(0.25);
        let now = Instant::now();

        // Prefer the queue-derived scheduling clock whenever it is available.
        if let Some(scheduling_target_seconds) = input.scheduling_target_seconds() {
            self.last_emit_instant = Some(now);
            self.media_seconds = scheduling_target_seconds;
            return self.media_seconds;
        }

        // Starved output: do not advance by wall-clock — that races video ahead of audible audio
        // (CPAL underruns, preroll edges). Tie to measured head when present, else hold.
        if input.critically_low_audio_buffer {
            self.last_emit_instant = Some(now);
            if let Some(target) = input.starved_hold_target_seconds() {
                self.media_seconds = target;
            }
            return self.media_seconds;
        }

        if let Some(last) = self.last_emit_instant {
            let elapsed = now.saturating_duration_since(last).as_secs_f64();
            if elapsed.is_finite() && elapsed > 0.0 {
                self.media_seconds = (self.media_seconds + elapsed * rate).max(0.0);
            }
        }
        self.last_emit_instant = Some(now);

        if self.media_seconds <= 0.0 {
            if let Some(hint) = input.hinted_seconds.filter(|v| v.is_finite() && *v >= 0.0) {
                self.media_seconds = hint;
            }
        }

        self.media_seconds
    }
}

#[cfg(test)]
mod tick_tests {
    use super::PlaybackClock;
    use crate::app::media::playback::runtime::sync_clock::{
        PlaybackClockInput, SyncClockSample, SyncClockSource,
    };
    use crate::app::media::state::TimingControls;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn starved_buffer_holds_without_observed_head() {
        let timing = Arc::new(TimingControls::default());
        let mut clock = PlaybackClock::new(30.0, 0, 0.0, timing, false);
        let synced = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(2.0, SyncClockSource::AudioEstimated),
            critically_low_audio_buffer: false,
            scheduling_lead_seconds: 0.0,
            ..PlaybackClockInput::default()
        };
        assert_eq!(clock.tick(synced), 2.0);
        thread::sleep(Duration::from_millis(80));
        let starved = PlaybackClockInput {
            critically_low_audio_buffer: true,
            ..PlaybackClockInput::default()
        };
        assert_eq!(clock.tick(starved), 2.0);
    }

    #[test]
    fn starved_buffer_tracks_observed_plus_lead() {
        let timing = Arc::new(TimingControls::default());
        let mut clock = PlaybackClock::new(30.0, 0, 0.0, timing, false);
        let synced = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(1.0, SyncClockSource::AudioEstimated),
            critically_low_audio_buffer: false,
            scheduling_lead_seconds: 0.0,
            ..PlaybackClockInput::default()
        };
        assert_eq!(clock.tick(synced), 1.0);
        thread::sleep(Duration::from_millis(80));
        let starved = PlaybackClockInput {
            critically_low_audio_buffer: true,
            estimated_audio_anchor: SyncClockSample::new(1.0, SyncClockSource::AudioEstimated),
            measured_audio_anchor: SyncClockSample::new(1.02, SyncClockSource::AudioMeasured),
            scheduling_lead_seconds: 0.01,
            ..PlaybackClockInput::default()
        };
        let out = clock.tick(starved);
        assert!((out - 1.03).abs() < 0.002, "expected ~1.03 got {out}");
    }
}
