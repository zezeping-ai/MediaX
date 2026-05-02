use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::state::AudioControls;
use rodio::Source;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use super::accumulator::AudioMeterAccumulator;
use super::shared::{publish_snapshot, SharedAudioMeter};

#[derive(Default)]
pub(crate) struct QueuedDurationTracker {
    total_micros: AtomicU64,
    blocks: Mutex<VecDeque<u64>>,
}

impl QueuedDurationTracker {
    pub(crate) fn push_block(&self, duration_micros: u64) {
        self.total_micros
            .fetch_add(duration_micros, Ordering::Relaxed);
        if let Ok(mut blocks) = self.blocks.lock() {
            blocks.push_back(duration_micros);
        }
    }

    pub(crate) fn finish_block(&self) {
        let Some(duration_micros) = self
            .blocks
            .lock()
            .ok()
            .and_then(|mut blocks| blocks.pop_front())
        else {
            return;
        };
        let _ = self.total_micros.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |total| Some(total.saturating_sub(duration_micros)),
        );
    }

    pub(crate) fn clear(&self) {
        self.total_micros.store(0, Ordering::Relaxed);
        if let Ok(mut blocks) = self.blocks.lock() {
            blocks.clear();
        }
    }

    pub(crate) fn queued_seconds(&self) -> f64 {
        (self.total_micros.load(Ordering::Relaxed) as f64) / 1_000_000.0
    }
}

pub(crate) struct MeteredSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    shared: SharedAudioMeter,
    controls: Arc<AudioControls>,
    accumulator: AudioMeterAccumulator,
    pending_frame: Vec<f32>,
    pending_frame_index: usize,
    queued_duration: Arc<QueuedDurationTracker>,
}

impl<S> MeteredSource<S>
where
    S: Source<Item = f32>,
{
    pub(crate) fn new(
        inner: S,
        shared: SharedAudioMeter,
        controls: Arc<AudioControls>,
        queued_duration: Arc<QueuedDurationTracker>,
    ) -> Self {
        let channels = inner.channels().get() as usize;
        let sample_rate = inner.sample_rate().get();
        Self {
            inner,
            shared,
            controls,
            accumulator: AudioMeterAccumulator::new(sample_rate, channels.max(1)),
            pending_frame: Vec::with_capacity(channels.max(1)),
            pending_frame_index: 0,
            queued_duration,
        }
    }

    fn flush_pending_snapshot(&mut self) {
        if let Some(snapshot) = self.accumulator.flush_snapshot() {
            publish_snapshot(&self.shared, snapshot);
        }
    }

    fn next_processed_sample(&mut self) -> Option<f32> {
        if self.pending_frame_index < self.pending_frame.len() {
            let sample = self.pending_frame[self.pending_frame_index];
            self.pending_frame_index += 1;
            return Some(sample);
        }

        let channel_count = usize::from(self.inner.channels().get().max(1));
        self.pending_frame.clear();
        while self.pending_frame.len() < channel_count {
            self.pending_frame.push(self.inner.next()?);
        }

        self.apply_live_controls();
        self.pending_frame_index = 1;
        self.pending_frame.first().copied()
    }

    fn apply_live_controls(&mut self) {
        let global_gain = if self.controls.muted() {
            0.0
        } else {
            self.controls.volume()
        };
        let left_gain = if self.controls.left_muted() {
            0.0
        } else {
            self.controls.left_volume()
        };
        let right_gain = if self.controls.right_muted() {
            0.0
        } else {
            self.controls.right_volume()
        };
        if self.pending_frame.is_empty() {
            return;
        }

        match self.controls.channel_routing() {
            PlaybackChannelRouting::Stereo => {}
            PlaybackChannelRouting::LeftToBoth if self.pending_frame.len() >= 2 => {
                self.pending_frame[1] = self.pending_frame[0];
            }
            PlaybackChannelRouting::RightToBoth if self.pending_frame.len() >= 2 => {
                self.pending_frame[0] = self.pending_frame[1];
            }
            _ => {}
        }

        for (index, sample) in self.pending_frame.iter_mut().enumerate() {
            let channel_gain = match index {
                0 => left_gain,
                1 => right_gain,
                _ => (left_gain + right_gain) * 0.5,
            };
            *sample *= global_gain * channel_gain;
        }
    }
}

impl<S> Iterator for MeteredSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let processed = self.next_processed_sample()?;
        if let Some(snapshot) = self.accumulator.push_sample(processed) {
            publish_snapshot(&self.shared, snapshot);
        }
        Some(processed)
    }
}

impl<S> Source for MeteredSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.inner.try_seek(pos)
    }
}

impl<S> Drop for MeteredSource<S>
where
    S: Source<Item = f32>,
{
    fn drop(&mut self) {
        // Network/compressed audio can arrive as many short queued buffers. Flush the
        // last partial meter window so short chunks still drive the live spectrum.
        self.flush_pending_snapshot();
        self.queued_duration.finish_block();
    }
}

#[cfg(test)]
mod tests {
    use super::QueuedDurationTracker;

    #[test]
    fn queued_duration_clear_does_not_underflow_on_late_drop() {
        let tracker = QueuedDurationTracker::default();
        tracker.push_block(120_000);
        tracker.clear();
        tracker.finish_block();
        assert_eq!(tracker.queued_seconds(), 0.0);
    }
}
