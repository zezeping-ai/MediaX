use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::state::AudioControls;
use rodio::Source;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::accumulator::AudioMeterAccumulator;
use super::shared::{publish_snapshot, SharedAudioMeter};

#[derive(Default)]
pub(crate) struct QueuedDurationTracker {
    state: Mutex<QueuedDurationState>,
}

#[derive(Default)]
struct QueuedDurationState {
    next_block_id: u64,
    blocks: VecDeque<QueuedBlock>,
    active_block: Option<ActiveQueuedBlock>,
}

#[derive(Clone, Copy)]
struct QueuedBlock {
    id: u64,
    wall_duration_micros: u64,
    media_start_seconds: Option<f64>,
    media_duration_seconds: f64,
}

#[derive(Clone, Copy)]
struct ActiveQueuedBlock {
    id: u64,
    wall_duration_micros: u64,
    media_start_seconds: Option<f64>,
    media_duration_seconds: f64,
    started_at: Instant,
}

impl QueuedDurationTracker {
    pub(crate) fn push_block(
        &self,
        wall_duration_micros: u64,
        media_start_seconds: Option<f64>,
        media_duration_seconds: f64,
    ) -> u64 {
        let Ok(mut state) = self.state.lock() else {
            return 0;
        };
        state.next_block_id = state.next_block_id.saturating_add(1);
        let id = state.next_block_id;
        state.blocks.push_back(QueuedBlock {
            id,
            wall_duration_micros,
            media_start_seconds,
            media_duration_seconds: media_duration_seconds.max(0.0),
        });
        id
    }

    pub(crate) fn mark_block_started(&self, block_id: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state
            .active_block
            .is_some_and(|active| active.id == block_id)
        {
            return;
        }
        let Some(block) = state.blocks.front().copied() else {
            return;
        };
        if block.id != block_id {
            return;
        }
        state.active_block = Some(ActiveQueuedBlock {
            id: block.id,
            wall_duration_micros: block.wall_duration_micros,
            media_start_seconds: block.media_start_seconds,
            media_duration_seconds: block.media_duration_seconds,
            started_at: Instant::now(),
        });
    }

    pub(crate) fn finish_block(&self, block_id: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state
            .active_block
            .is_some_and(|active| active.id == block_id)
        {
            state.active_block = None;
        }
        if let Some(position) = state.blocks.iter().position(|block| block.id == block_id) {
            state.blocks.remove(position);
        }
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.blocks.clear();
            state.active_block = None;
        }
    }

    pub(crate) fn queued_seconds(&self) -> f64 {
        let Ok(state) = self.state.lock() else {
            return 0.0;
        };
        let mut total_micros = 0u128;
        for block in &state.blocks {
            if let Some(active) = state.active_block.filter(|active| active.id == block.id) {
                let elapsed_micros = active.started_at.elapsed().as_micros() as u64;
                let remaining_micros = active.wall_duration_micros.saturating_sub(elapsed_micros);
                total_micros = total_micros.saturating_add(u128::from(remaining_micros));
            } else {
                total_micros = total_micros.saturating_add(u128::from(block.wall_duration_micros));
            }
        }
        (total_micros as f64) / 1_000_000.0
    }

    pub(crate) fn playback_head_seconds(&self, extra_latency_seconds: f64) -> Option<f64> {
        let Ok(state) = self.state.lock() else {
            return None;
        };
        let front_block = state.blocks.front().copied()?;
        let extra_latency_seconds = extra_latency_seconds.max(0.0);
        if let Some(active) = state
            .active_block
            .filter(|active| active.id == front_block.id)
        {
            let media_start_seconds = active
                .media_start_seconds
                .filter(|value| value.is_finite() && *value >= 0.0)?;
            let wall_duration_seconds = (active.wall_duration_micros as f64) / 1_000_000.0;
            if wall_duration_seconds <= 0.0 {
                return Some(media_start_seconds);
            }
            // Samples are considered "started" when rodio pulls them, but the audible device
            // head is still behind by the downstream output latency. Subtract that reserve so the
            // reported playback head tracks what users can actually hear, not just what the mixer
            // has already consumed.
            let consumed_wall_seconds = (active.started_at.elapsed().as_secs_f64()
                - extra_latency_seconds)
                .clamp(0.0, wall_duration_seconds);
            let progress = (consumed_wall_seconds / wall_duration_seconds).clamp(0.0, 1.0);
            return Some(
                (media_start_seconds + active.media_duration_seconds.max(0.0) * progress).max(0.0),
            );
        }
        front_block
            .media_start_seconds
            .filter(|value| value.is_finite() && *value >= 0.0)
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
    queued_block_id: u64,
    playback_started: bool,
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
        queued_block_id: u64,
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
            queued_block_id,
            playback_started: false,
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
        if !self.playback_started {
            self.queued_duration
                .mark_block_started(self.queued_block_id);
            self.playback_started = true;
        }
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
        self.queued_duration.finish_block(self.queued_block_id);
    }
}

#[cfg(test)]
mod tests {
    use super::QueuedDurationTracker;

    #[test]
    fn queued_duration_clear_does_not_underflow_on_late_drop() {
        let tracker = QueuedDurationTracker::default();
        let block_id = tracker.push_block(120_000, Some(3.0), 0.12);
        tracker.clear();
        tracker.finish_block(block_id);
        assert_eq!(tracker.queued_seconds(), 0.0);
    }
}
