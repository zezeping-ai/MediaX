use crate::app::media::model::PlaybackChannelRouting;
use crate::app::media::state::AudioControls;
use rodio::Source;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use super::accumulator::AudioMeterAccumulator;
use super::shared::{publish_snapshot, SharedAudioMeter};

pub(crate) struct MeteredSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    shared: SharedAudioMeter,
    controls: Arc<AudioControls>,
    accumulator: AudioMeterAccumulator,
    pending_input_frame: Vec<f32>,
    pending_output_frame: VecDeque<f32>,
}

impl<S> MeteredSource<S>
where
    S: Source<Item = f32>,
{
    pub(crate) fn new(inner: S, shared: SharedAudioMeter, controls: Arc<AudioControls>) -> Self {
        let channels = inner.channels().get() as usize;
        let sample_rate = inner.sample_rate().get();
        Self {
            inner,
            shared,
            controls,
            accumulator: AudioMeterAccumulator::new(sample_rate, channels.max(1)),
            pending_input_frame: Vec::with_capacity(channels.max(1)),
            pending_output_frame: VecDeque::with_capacity(channels.max(1)),
        }
    }

    fn flush_pending_snapshot(&mut self) {
        if let Some(snapshot) = self.accumulator.flush_snapshot() {
            publish_snapshot(&self.shared, snapshot);
        }
    }

    fn next_processed_sample(&mut self) -> Option<f32> {
        if let Some(sample) = self.pending_output_frame.pop_front() {
            return Some(sample);
        }

        let channel_count = usize::from(self.inner.channels().get().max(1));
        while self.pending_input_frame.len() < channel_count {
            self.pending_input_frame.push(self.inner.next()?);
        }

        let routed_frame = self.apply_live_controls(&self.pending_input_frame);
        self.pending_input_frame.clear();
        self.pending_output_frame.extend(routed_frame);
        self.pending_output_frame.pop_front()
    }

    fn apply_live_controls(&self, frame: &[f32]) -> Vec<f32> {
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
        let mut output = frame.to_vec();
        if output.is_empty() {
            return output;
        }

        match self.controls.channel_routing() {
            PlaybackChannelRouting::Stereo => {}
            PlaybackChannelRouting::LeftToBoth if output.len() >= 2 => {
                output[1] = output[0];
            }
            PlaybackChannelRouting::RightToBoth if output.len() >= 2 => {
                output[0] = output[1];
            }
            _ => {}
        }

        for (index, sample) in output.iter_mut().enumerate() {
            let channel_gain = match index {
                0 => left_gain,
                1 => right_gain,
                _ => (left_gain + right_gain) * 0.5,
            };
            *sample *= global_gain * channel_gain;
        }

        output
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
    }
}
