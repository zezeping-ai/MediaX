use crate::app::media::player::state::TimingControls;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct AudioClock {
    pub anchor_instant: Instant,
    pub anchor_media_seconds: f64,
    pub anchor_rate: f64,
}

impl AudioClock {
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
}

pub struct PlaybackClock {
    pub frame_duration: Duration,
    pub last_emit_instant: Option<Instant>,
    pub media_seconds: f64,
    pub timing_controls: Arc<TimingControls>,
}

#[derive(Default)]
pub struct FpsWindow {
    pub started_at: Option<Instant>,
    pub frames: u32,
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
        }
    }

    pub fn reset_to(&mut self, media_seconds: f64) {
        self.media_seconds = media_seconds.max(0.0);
        self.last_emit_instant = None;
    }

    pub fn playback_rate(&self) -> f64 {
        self.timing_controls.playback_rate() as f64
    }

    pub fn tick(
        &mut self,
        hinted_seconds: Option<f64>,
        audio_position_seconds: Option<f64>,
        audio_queue_depth_sources: Option<usize>,
        audio_allowed_lead_seconds: f64,
    ) -> f64 {
        let rate = self.playback_rate().max(0.25);
        let low_audio_buffer = audio_queue_depth_sources
            .map(|depth| depth < 3)
            .unwrap_or(false);
        let now = Instant::now();

        if let Some(last) = self.last_emit_instant {
            let elapsed = now.saturating_duration_since(last).as_secs_f64();
            if elapsed.is_finite() && elapsed > 0.0 {
                self.media_seconds = (self.media_seconds + elapsed * rate).max(0.0);
            }
        }
        self.last_emit_instant = Some(now);

        if !low_audio_buffer {
            if let Some(audio_seconds) =
                audio_position_seconds.filter(|v| v.is_finite() && *v >= 0.0)
            {
                let allowed_lead_seconds = audio_allowed_lead_seconds.max(0.0);
                self.media_seconds = (audio_seconds + allowed_lead_seconds).max(0.0);
                return self.media_seconds;
            }
        }

        if self.media_seconds <= 0.0 {
            if let Some(hint) = hinted_seconds.filter(|v| v.is_finite() && *v >= 0.0) {
                self.media_seconds = hint;
            }
        }

        self.media_seconds
    }
}
