use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::format;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use rodio::{MixerDeviceSink, Player};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

pub(crate) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output_sample_format: AudioOutputSampleFormat,
    pub output: AudioOutput,
    pub stats: AudioStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AudioOutputSampleFormat {
    F32Packed,
    I16Packed,
}

impl AudioOutputSampleFormat {
    pub fn ffmpeg_sample_format(self) -> format::Sample {
        match self {
            Self::F32Packed => format::Sample::F32(format::sample::Type::Packed),
            Self::I16Packed => format::Sample::I16(format::sample::Type::Packed),
        }
    }

    pub fn debug_label(self) -> &'static str {
        match self {
            Self::F32Packed => "f32-packed",
            Self::I16Packed => "i16-packed",
        }
    }
}

pub(crate) struct AudioOutput {
    pub _stream: MixerDeviceSink,
    pub player: Player,
    pub controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
    pub meter_stop_flag: Arc<AtomicBool>,
    pub meter_thread: Option<JoinHandle<()>>,
    pub meter_shared: super::meter::SharedAudioMeter,
}

#[derive(Default)]
pub(crate) struct AudioStats {
    pub packets: u64,
    pub decoded_frames: u64,
    pub queued_samples: u64,
    pub underrun_count: u64,
    pub last_debug_instant: Option<Instant>,
}
