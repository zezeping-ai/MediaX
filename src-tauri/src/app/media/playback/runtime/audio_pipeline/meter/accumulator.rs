use std::collections::VecDeque;

use super::shared::AudioMeterSnapshot;
use super::spectrum::AUDIO_SPECTRUM_WINDOW;

const AUDIO_METER_EMIT_HZ: u32 = 30;

pub(crate) struct AudioMeterAccumulator {
    sample_rate: u32,
    channels: usize,
    frames_per_emit: usize,
    frames_since_emit: usize,
    pending_frame: Vec<f32>,
    left_window: VecDeque<f32>,
    right_window: VecDeque<f32>,
    interval_left_peak: f32,
    interval_right_peak: f32,
}

impl AudioMeterAccumulator {
    pub(crate) fn new(sample_rate: u32, channels: usize) -> Self {
        let frames_per_emit = (sample_rate / AUDIO_METER_EMIT_HZ).max(1) as usize;
        Self {
            sample_rate,
            channels,
            frames_per_emit,
            frames_since_emit: 0,
            pending_frame: Vec::with_capacity(channels),
            left_window: VecDeque::with_capacity(AUDIO_SPECTRUM_WINDOW),
            right_window: VecDeque::with_capacity(AUDIO_SPECTRUM_WINDOW),
            interval_left_peak: 0.0,
            interval_right_peak: 0.0,
        }
    }

    pub(crate) fn push_sample(&mut self, sample: f32) -> Option<AudioMeterSnapshot> {
        self.pending_frame.push(sample);
        if self.pending_frame.len() < self.channels {
            return None;
        }
        let (left, right) = self.take_pending_frame_pair();
        self.capture_frame(left, right);
        self.frames_since_emit = self.frames_since_emit.saturating_add(1);
        if self.frames_since_emit < self.frames_per_emit {
            return None;
        }
        self.frames_since_emit = 0;
        Some(self.build_snapshot())
    }

    pub(crate) fn flush_snapshot(&mut self) -> Option<AudioMeterSnapshot> {
        if self.left_window.is_empty()
            && self.right_window.is_empty()
            && self.pending_frame.is_empty()
        {
            return None;
        }
        if !self.pending_frame.is_empty() {
            let (left, right) = self.take_pending_frame_pair();
            self.capture_frame(left, right);
        }
        self.frames_since_emit = 0;
        Some(self.build_snapshot())
    }

    fn take_pending_frame_pair(&mut self) -> (f32, f32) {
        let left = self.pending_frame.first().copied().unwrap_or(0.0);
        let right = if self.channels > 1 {
            self.pending_frame.get(1).copied().unwrap_or(left)
        } else {
            left
        };
        self.pending_frame.clear();
        (left, right)
    }

    fn capture_frame(&mut self, left: f32, right: f32) {
        Self::push_window_sample(&mut self.left_window, left);
        Self::push_window_sample(&mut self.right_window, right);
        self.interval_left_peak = self.interval_left_peak.max(left.abs());
        self.interval_right_peak = self.interval_right_peak.max(right.abs());
    }

    fn build_snapshot(&mut self) -> AudioMeterSnapshot {
        let snapshot = AudioMeterSnapshot {
            sample_rate: self.sample_rate,
            channels: self.channels as u16,
            left_peak: self.interval_left_peak.clamp(0.0, 1.0),
            right_peak: self.interval_right_peak.clamp(0.0, 1.0),
            left_samples: self.left_window.iter().copied().collect(),
            right_samples: self.right_window.iter().copied().collect(),
        };
        self.interval_left_peak = 0.0;
        self.interval_right_peak = 0.0;
        snapshot
    }

    fn push_window_sample(window: &mut VecDeque<f32>, sample: f32) {
        if window.len() >= AUDIO_SPECTRUM_WINDOW {
            let _ = window.pop_front();
        }
        window.push_back(sample);
    }
}
