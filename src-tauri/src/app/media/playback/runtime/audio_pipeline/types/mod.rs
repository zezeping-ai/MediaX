mod output_format;
mod stats;

use crate::app::media::playback::rate::{discontinuity_smoothing_profile, PlaybackRate};
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;

pub(crate) use output_format::AudioOutputSampleFormat;
pub(crate) use stats::AudioStats;

pub(crate) struct StagedOutputPcm {
    pub blocks: Vec<Vec<f32>>,
    pub scratch: Vec<f32>,
}

pub(crate) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output_sample_rate: u32,
    pub output_sample_format: AudioOutputSampleFormat,
    pub time_stretch: super::time_stretch::AudioTimeStretch,
    pub output: super::output::AudioOutput,
    pub stats: AudioStats,
    pub(crate) scratch_pcm: Vec<f32>,
    pub(crate) discontinuity_fade_in_frames_remaining: usize,
    pub(crate) discontinuity_fade_in_total_frames: usize,
    pub(crate) output_staging_channels: u16,
    pub(crate) output_staging_samples: Vec<f32>,
    pub(crate) output_staging_start: usize,
    pub(crate) output_media_cursor_seconds: Option<f64>,
    pub(crate) recent_output_tail_channels: u16,
    pub(crate) recent_output_tail_samples: Vec<f32>,
    pub(crate) discontinuity_crossfade_tail_channels: u16,
    pub(crate) discontinuity_crossfade_tail_samples: Vec<f32>,
    pub(crate) discontinuity_crossfade_frames: usize,
}

impl AudioPipeline {
    pub fn reset_processing_state(&mut self) {
        self.time_stretch.reset();
    }

    pub fn clear_output_queue(&self) {
        self.output.clear_queue();
    }

    pub fn restart_after_discontinuity(
        &mut self,
        previous_rate: PlaybackRate,
        next_rate: PlaybackRate,
        preserve_output_queue: bool,
    ) {
        self.reset_processing_state();
        if preserve_output_queue {
            self.stats.intentional_refill_pending = false;
            self.discontinuity_fade_in_total_frames = 0;
            self.discontinuity_fade_in_frames_remaining = 0;
            self.discontinuity_crossfade_frames = 0;
            self.discontinuity_crossfade_tail_channels = 0;
            self.discontinuity_crossfade_tail_samples.clear();
            self.recent_output_tail_channels = 0;
            self.recent_output_tail_samples.clear();
        } else {
            self.clear_output_queue();
            self.stats.intentional_refill_pending = true;
            let smoothing = discontinuity_smoothing_profile(previous_rate, next_rate);
            self.discontinuity_fade_in_total_frames = smoothing.fade_in_frames;
            self.discontinuity_fade_in_frames_remaining = smoothing.fade_in_frames;
            self.discontinuity_crossfade_frames = smoothing.crossfade_frames;
            self.discontinuity_crossfade_tail_channels = self.recent_output_tail_channels;
            self.discontinuity_crossfade_tail_samples =
                std::mem::take(&mut self.recent_output_tail_samples);
        }
        self.output_staging_channels = 0;
        self.output_staging_samples.clear();
        self.output_staging_start = 0;
        self.output_media_cursor_seconds = None;
    }

    pub fn mark_refill_completed(&mut self) {
        self.stats.intentional_refill_pending = false;
    }

    pub fn apply_discontinuity_smoothing(&mut self, pcm: &mut [f32], channels: u16) {
        let channel_count = usize::from(channels.max(1));
        if self.discontinuity_fade_in_frames_remaining == 0 || channel_count == 0 || pcm.is_empty()
        {
            self.apply_discontinuity_crossfade(pcm, channels);
            return;
        }
        self.apply_discontinuity_crossfade(pcm, channels);
        let available_frames = pcm.len() / channel_count;
        let fade_frames = available_frames.min(self.discontinuity_fade_in_frames_remaining);
        if fade_frames == 0 {
            return;
        }
        let total_fade_frames = self.discontinuity_fade_in_total_frames.max(1);
        let progressed_frames =
            total_fade_frames.saturating_sub(self.discontinuity_fade_in_frames_remaining);
        for frame_index in 0..fade_frames {
            let absolute_index = progressed_frames + frame_index;
            let gain = ((absolute_index + 1) as f32) / (total_fade_frames as f32);
            let frame_offset = frame_index * channel_count;
            for sample in &mut pcm[frame_offset..frame_offset + channel_count] {
                *sample *= gain;
            }
        }
        self.discontinuity_fade_in_frames_remaining = self
            .discontinuity_fade_in_frames_remaining
            .saturating_sub(fade_frames);
    }

    pub fn stage_output_pcm_owned(
        &mut self,
        mut pcm: Vec<f32>,
        channels: u16,
        staging_frames: usize,
        force_flush_partial: bool,
        min_partial_flush_frames: usize,
    ) -> StagedOutputPcm {
        if pcm.is_empty() || channels == 0 {
            return StagedOutputPcm {
                blocks: Vec::new(),
                scratch: pcm,
            };
        }
        if self.output_staging_channels != channels {
            self.output_staging_channels = channels;
            self.output_staging_samples.clear();
            self.output_staging_start = 0;
        }
        let samples_per_block = staging_frames.max(1).saturating_mul(usize::from(channels));
        let min_partial_flush_samples = min_partial_flush_frames
            .max(1)
            .saturating_mul(usize::from(channels));
        if self.available_staging_samples() == 0 {
            return self.stage_output_pcm_direct(
                pcm,
                channels,
                samples_per_block,
                force_flush_partial,
                min_partial_flush_samples,
            );
        }

        self.output_staging_samples.extend_from_slice(&pcm);
        pcm.clear();
        let mut blocks = Vec::new();
        while self.available_staging_samples() >= samples_per_block {
            let block = self.take_staged_block(samples_per_block);
            self.remember_output_tail(&block, channels);
            blocks.push(block);
        }
        self.compact_staging_buffer_if_needed();
        if force_flush_partial
            && self.available_staging_samples() >= min_partial_flush_samples
        {
            let block = self.take_remaining_staging_samples();
            self.remember_output_tail(&block, channels);
            blocks.push(block);
        }
        StagedOutputPcm {
            blocks,
            scratch: pcm,
        }
    }

    pub fn flush_staged_output_pcm(&mut self) -> Option<(u16, Vec<f32>)> {
        if self.available_staging_samples() == 0 || self.output_staging_channels == 0 {
            return None;
        }
        let channels = self.output_staging_channels;
        let block = self.take_remaining_staging_samples();
        self.remember_output_tail(&block, channels);
        Some((channels, block))
    }

    fn apply_discontinuity_crossfade(&mut self, pcm: &mut [f32], channels: u16) {
        let channel_count = usize::from(channels.max(1));
        if self.discontinuity_crossfade_tail_samples.is_empty()
            || self.discontinuity_crossfade_tail_channels != channels
            || channel_count == 0
        {
            self.discontinuity_crossfade_tail_samples.clear();
            self.discontinuity_crossfade_tail_channels = 0;
            return;
        }
        let available_frames = pcm.len() / channel_count;
        let tail_frames = self.discontinuity_crossfade_tail_samples.len() / channel_count;
        let crossfade_frames = available_frames
            .min(tail_frames)
            .min(self.discontinuity_crossfade_frames.max(1));
        if crossfade_frames == 0 {
            self.discontinuity_crossfade_tail_samples.clear();
            self.discontinuity_crossfade_tail_channels = 0;
            return;
        }
        let tail_start = self
            .discontinuity_crossfade_tail_samples
            .len()
            .saturating_sub(crossfade_frames * channel_count);
        for frame_index in 0..crossfade_frames {
            let fade_in = ((frame_index + 1) as f32) / (crossfade_frames as f32);
            let fade_out = 1.0 - fade_in;
            let head_offset = frame_index * channel_count;
            let tail_offset = tail_start + head_offset;
            for channel_index in 0..channel_count {
                pcm[head_offset + channel_index] = self.discontinuity_crossfade_tail_samples
                    [tail_offset + channel_index]
                    * fade_out
                    + pcm[head_offset + channel_index] * fade_in;
            }
        }
        self.discontinuity_crossfade_tail_samples.clear();
        self.discontinuity_crossfade_tail_channels = 0;
        self.discontinuity_fade_in_frames_remaining = self
            .discontinuity_fade_in_frames_remaining
            .saturating_sub(crossfade_frames);
    }

    fn remember_output_tail(&mut self, pcm: &[f32], channels: u16) {
        let channel_count = usize::from(channels.max(1));
        if pcm.is_empty() || channel_count == 0 {
            return;
        }
        let tail_samples = self
            .discontinuity_crossfade_frames
            .max(1)
            .saturating_mul(channel_count);
        let start = pcm.len().saturating_sub(tail_samples);
        self.recent_output_tail_channels = channels;
        self.recent_output_tail_samples.clear();
        self.recent_output_tail_samples
            .extend_from_slice(&pcm[start..]);
    }

    fn available_staging_samples(&self) -> usize {
        self.output_staging_samples
            .len()
            .saturating_sub(self.output_staging_start)
    }

    fn stage_output_pcm_direct(
        &mut self,
        mut pcm: Vec<f32>,
        channels: u16,
        samples_per_block: usize,
        force_flush_partial: bool,
        min_partial_flush_samples: usize,
    ) -> StagedOutputPcm {
        let mut blocks = Vec::new();
        while pcm.len() >= samples_per_block && samples_per_block > 0 {
            let remainder = pcm.split_off(samples_per_block);
            let block = std::mem::replace(&mut pcm, remainder);
            self.remember_output_tail(&block, channels);
            blocks.push(block);
        }
        if force_flush_partial && pcm.len() >= min_partial_flush_samples {
            self.remember_output_tail(&pcm, channels);
            blocks.push(pcm);
            return StagedOutputPcm {
                blocks,
                scratch: Vec::new(),
            };
        }
        if !pcm.is_empty() {
            self.output_staging_channels = channels;
            self.output_staging_samples = pcm;
            self.output_staging_start = 0;
            return StagedOutputPcm {
                blocks,
                scratch: Vec::new(),
            };
        }
        StagedOutputPcm {
            blocks,
            scratch: pcm,
        }
    }

    fn compact_staging_buffer_if_needed(&mut self) {
        if self.output_staging_start == 0 {
            return;
        }
        if self.output_staging_start < self.output_staging_samples.len() / 2 {
            return;
        }
        self.output_staging_samples
            .drain(..self.output_staging_start);
        self.output_staging_start = 0;
    }

    fn take_remaining_staging_samples(&mut self) -> Vec<f32> {
        let remaining = self.available_staging_samples();
        if remaining == 0 {
            self.output_staging_samples.clear();
            self.output_staging_start = 0;
            return Vec::new();
        }
        let mut block = Vec::with_capacity(remaining);
        block.extend_from_slice(&self.output_staging_samples[self.output_staging_start..]);
        self.output_staging_samples.clear();
        self.output_staging_start = 0;
        block
    }

    fn take_staged_block(&mut self, samples_per_block: usize) -> Vec<f32> {
        if self.output_staging_start > 0 {
            self.compact_staging_buffer_force();
        }
        if self.output_staging_samples.len() == samples_per_block {
            self.output_staging_start = 0;
            return std::mem::take(&mut self.output_staging_samples);
        }
        let remainder = self.output_staging_samples.split_off(samples_per_block);
        let block = std::mem::replace(&mut self.output_staging_samples, remainder);
        self.output_staging_start = 0;
        block
    }

    fn compact_staging_buffer_force(&mut self) {
        if self.output_staging_start == 0 {
            return;
        }
        self.output_staging_samples
            .drain(..self.output_staging_start);
        self.output_staging_start = 0;
    }
}
