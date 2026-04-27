mod output_format;
mod stats;

use ffmpeg_next::software::resampling::context::Context as ResamplingContext;

pub(crate) use output_format::AudioOutputSampleFormat;
pub(crate) use stats::AudioStats;

const DISCONTINUITY_FADE_IN_FRAMES: usize = 256;
const DISCONTINUITY_CROSSFADE_FRAMES: usize = 192;
const OUTPUT_STAGING_FRAMES: usize = 1024;

pub(crate) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output_sample_format: AudioOutputSampleFormat,
    pub time_stretch: super::time_stretch::AudioTimeStretch,
    pub output: super::output::AudioOutput,
    pub stats: AudioStats,
    pub(crate) discontinuity_fade_in_frames_remaining: usize,
    pub(crate) output_staging_channels: u16,
    pub(crate) output_staging_samples: Vec<f32>,
    pub(crate) recent_output_tail_channels: u16,
    pub(crate) recent_output_tail_samples: Vec<f32>,
    pub(crate) discontinuity_crossfade_tail_channels: u16,
    pub(crate) discontinuity_crossfade_tail_samples: Vec<f32>,
}

impl AudioPipeline {
    pub fn reset_processing_state(&mut self) {
        self.time_stretch.reset();
    }

    pub fn clear_output_queue(&self) {
        self.output.clear_queue();
    }

    pub fn restart_after_discontinuity(&mut self) {
        self.reset_processing_state();
        self.clear_output_queue();
        self.stats.intentional_refill_pending = true;
        self.discontinuity_fade_in_frames_remaining = DISCONTINUITY_FADE_IN_FRAMES;
        self.output_staging_channels = 0;
        self.output_staging_samples.clear();
        self.discontinuity_crossfade_tail_channels = self.recent_output_tail_channels;
        self.discontinuity_crossfade_tail_samples = self.recent_output_tail_samples.clone();
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
        let progressed_frames =
            DISCONTINUITY_FADE_IN_FRAMES.saturating_sub(self.discontinuity_fade_in_frames_remaining);
        for frame_index in 0..fade_frames {
            let absolute_index = progressed_frames + frame_index;
            let gain = ((absolute_index + 1) as f32) / (DISCONTINUITY_FADE_IN_FRAMES as f32);
            let frame_offset = frame_index * channel_count;
            for sample in &mut pcm[frame_offset..frame_offset + channel_count] {
                *sample *= gain;
            }
        }
        self.discontinuity_fade_in_frames_remaining =
            self.discontinuity_fade_in_frames_remaining.saturating_sub(fade_frames);
    }

    pub fn stage_output_pcm(
        &mut self,
        pcm: &[f32],
        channels: u16,
        force_flush_partial: bool,
    ) -> Vec<Vec<f32>> {
        if pcm.is_empty() || channels == 0 {
            return Vec::new();
        }
        if self.output_staging_channels != channels {
            self.output_staging_channels = channels;
            self.output_staging_samples.clear();
        }
        self.output_staging_samples.extend_from_slice(pcm);
        let mut blocks = Vec::new();
        let samples_per_block = OUTPUT_STAGING_FRAMES.saturating_mul(usize::from(channels));
        while self.output_staging_samples.len() >= samples_per_block {
            let block: Vec<f32> = self.output_staging_samples.drain(..samples_per_block).collect();
            self.remember_output_tail(&block, channels);
            blocks.push(block);
        }
        if force_flush_partial && !self.output_staging_samples.is_empty() {
            let block = std::mem::take(&mut self.output_staging_samples);
            self.remember_output_tail(&block, channels);
            blocks.push(block);
        }
        blocks
    }

    pub fn flush_staged_output_pcm(&mut self) -> Option<(u16, Vec<f32>)> {
        if self.output_staging_samples.is_empty() || self.output_staging_channels == 0 {
            return None;
        }
        let channels = self.output_staging_channels;
        let block = std::mem::take(&mut self.output_staging_samples);
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
            .min(DISCONTINUITY_CROSSFADE_FRAMES);
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
        let tail_samples = DISCONTINUITY_CROSSFADE_FRAMES.saturating_mul(channel_count);
        let start = pcm.len().saturating_sub(tail_samples);
        self.recent_output_tail_channels = channels;
        self.recent_output_tail_samples.clear();
        self.recent_output_tail_samples.extend_from_slice(&pcm[start..]);
    }
}
