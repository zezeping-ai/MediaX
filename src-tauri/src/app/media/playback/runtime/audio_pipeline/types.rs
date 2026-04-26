use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use rodio::{MixerDeviceSink, Player};
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output: AudioOutput,
    pub stats: AudioStats,
}

pub(crate) struct AudioOutput {
    pub _stream: MixerDeviceSink,
    pub player: Player,
    pub controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
}

#[derive(Default)]
pub(crate) struct AudioStats {
    pub packets: u64,
    pub decoded_frames: u64,
    pub queued_samples: u64,
    pub underrun_count: u64,
    pub last_debug_instant: Option<Instant>,
}
