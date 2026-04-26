use super::audio::clamp_playback_rate;
use super::{emit_debug, METRICS_EMIT_INTERVAL_MS};
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::format::sample::Type as SampleType;
use ffmpeg_next::frame;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use rodio::{buffer::SamplesBuffer, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

use super::clock::AudioClock;

pub const DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD: usize = 8;

pub(super) struct AudioPipeline {
    pub stream_index: usize,
    pub decoder: ffmpeg_next::decoder::Audio,
    pub time_base: ffmpeg_next::Rational,
    pub resampler: ResamplingContext,
    pub output: AudioOutput,
    pub stats: AudioStats,
}

pub(super) struct AudioOutput {
    _stream: MixerDeviceSink,
    pub player: Player,
    pub controls: Arc<AudioControls>,
    timing_controls: Arc<TimingControls>,
}

#[derive(Default)]
pub(super) struct AudioStats {
    pub packets: u64,
    pub decoded_frames: u64,
    pub queued_samples: u64,
    pub underrun_count: u64,
    pub last_debug_instant: Option<Instant>,
}

impl AudioOutput {
    fn new(
        controls: Arc<AudioControls>,
        timing_controls: Arc<TimingControls>,
    ) -> Result<Self, String> {
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("open default audio output failed: {err}"))?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let output = Self {
            _stream: stream,
            player,
            controls,
            timing_controls,
        };
        output.player.play();
        output.apply_controls();
        Ok(output)
    }

    fn apply_controls(&self) {
        let volume = if self.controls.muted() {
            0.0
        } else {
            self.controls.volume()
        };
        self.player.set_volume(volume);
        self.player
            .set_speed(clamp_playback_rate(self.timing_controls.playback_rate()));
    }

    fn append_pcm_i16(&self, sample_rate: u32, channels: u16, pcm: &[i16]) {
        let Some(channels) = NonZeroU16::new(channels) else {
            return;
        };
        let Some(sample_rate) = NonZeroU32::new(sample_rate) else {
            return;
        };
        if pcm.is_empty() {
            return;
        }
        self.apply_controls();
        let samples: Vec<f32> = pcm
            .iter()
            .map(|sample| (*sample as f32) / (i16::MAX as f32))
            .collect();
        self.player
            .append(SamplesBuffer::new(channels, sample_rate, samples));
    }
}

pub(super) fn build_audio_pipeline(
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
) -> Result<Option<AudioPipeline>, String> {
    let Some(stream_index) = audio_stream_index else {
        return Ok(None);
    };
    let input_stream = input_ctx
        .streams()
        .find(|stream| stream.index() == stream_index)
        .ok_or_else(|| "audio stream index not found".to_string())?;
    let audio_context = codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|err| format!("audio decoder context failed: {err}"))?;
    let decoder = audio_context
        .decoder()
        .audio()
        .map_err(|err| format!("audio decoder create failed: {err}"))?;
    let channel_layout = if decoder.channel_layout().is_empty() {
        ChannelLayout::default(decoder.channels().into())
    } else {
        decoder.channel_layout()
    };
    let resampler = ResamplingContext::get(
        decoder.format(),
        channel_layout,
        decoder.rate(),
        ffmpeg_next::format::Sample::I16(SampleType::Packed),
        channel_layout,
        decoder.rate(),
    )
    .map_err(|err| format!("audio resampler create failed: {err}"))?;
    let output = AudioOutput::new(audio_controls.clone(), timing_controls.clone())?;
    Ok(Some(AudioPipeline {
        stream_index,
        decoder,
        time_base: input_stream.time_base(),
        resampler,
        output,
        stats: AudioStats::default(),
    }))
}

pub(super) fn drain_audio_frames(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    audio_clock: &mut Option<AudioClock>,
    audio_queue_depth_sources: &mut Option<usize>,
    active_seek_target_seconds: &mut Option<f64>,
) -> Result<(), String> {
    audio_state.stats.packets = audio_state.stats.packets.saturating_add(1);
    let mut decoded = frame::Audio::empty();
    while audio_state.decoder.receive_frame(&mut decoded).is_ok() {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        audio_state.stats.decoded_frames = audio_state.stats.decoded_frames.saturating_add(1);
        let mut converted = frame::Audio::empty();
        audio_state
            .resampler
            .run(&decoded, &mut converted)
            .map_err(|err| format!("audio resample failed: {err}"))?;
        let channels = converted.channels().max(1) as usize;
        let samples_per_channel = converted.samples();
        let total_samples = samples_per_channel.saturating_mul(channels);
        if total_samples == 0 {
            continue;
        }
        let bytes_per_sample = std::mem::size_of::<i16>();
        let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
        let data = converted.data(0);
        if data.is_empty() {
            continue;
        }
        let clamped_bytes = expected_bytes.min(data.len());
        if clamped_bytes < bytes_per_sample {
            continue;
        }
        audio_state
            .output
            .player
            .set_speed(clamp_playback_rate(timing_controls.playback_rate()));
        if audio_state.output.player.len() == 0 {
            audio_state.stats.underrun_count = audio_state.stats.underrun_count.saturating_add(1);
        }
        if audio_state.output.player.is_paused() {
            audio_state.output.player.play();
            emit_debug(
                app,
                "audio_resume",
                "audio player resumed from paused state",
            );
        }
        if let Some(seconds) =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), audio_state.time_base)
                .filter(|value| value.is_finite() && *value >= 0.0)
        {
            if let Some(target) = *active_seek_target_seconds {
                if seconds + 0.03 < target {
                    continue;
                }
                *active_seek_target_seconds = None;
            }
            if audio_clock.is_none() {
                *audio_clock = Some(AudioClock {
                    anchor_instant: Instant::now(),
                    anchor_media_seconds: seconds,
                    anchor_rate: timing_controls.playback_rate().max(0.25) as f64,
                });
            }
        }
        let pcm: Vec<i16> = data[..clamped_bytes]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        audio_state.stats.queued_samples = audio_state
            .stats
            .queued_samples
            .saturating_add(pcm.len() as u64);
        audio_state
            .output
            .append_pcm_i16(converted.rate(), converted.channels(), &pcm);
        *audio_queue_depth_sources = Some(audio_state.output.player.len());
        let now = Instant::now();
        let should_emit = audio_state
            .stats
            .last_debug_instant
            .map(|last| {
                now.saturating_duration_since(last)
                    >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS)
            })
            .unwrap_or(true);
        if should_emit {
            audio_state.stats.last_debug_instant = Some(now);
            emit_debug(
                app,
                "audio_output",
                format!(
                    "volume={:.2} muted={} rate={:.2} queue_sources={}",
                    audio_state.output.controls.volume(),
                    audio_state.output.controls.muted(),
                    timing_controls.playback_rate(),
                    audio_state.output.player.len()
                ),
            );
            emit_debug(
                app,
                "audio_stats",
                format!(
                    "packets={} frames={} queued_samples={} underruns={} queue_sources={} rate={:.2} channels={} samples_per_ch={} bytes={} pts={}",
                    audio_state.stats.packets,
                    audio_state.stats.decoded_frames,
                    audio_state.stats.queued_samples,
                    audio_state.stats.underrun_count,
                    audio_state.output.player.len(),
                    timing_controls.playback_rate(),
                    channels,
                    samples_per_channel,
                    clamped_bytes,
                    audio_clock
                        .as_ref()
                        .map(|clock| clock.now_seconds())
                        .map(|v| format!("{v:.3}s"))
                        .unwrap_or_else(|| "n/a".to_string()),
                ),
            );
        }
    }
    Ok(())
}
