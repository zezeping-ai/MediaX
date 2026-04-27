mod output_format;
mod stats;

use ffmpeg_next::software::resampling::context::Context as ResamplingContext;

pub(crate) use output_format::AudioOutputSampleFormat;
pub(crate) use stats::AudioStats;

pub(crate) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output_sample_format: AudioOutputSampleFormat,
    pub output: super::output::AudioOutput,
    pub stats: AudioStats,
}
