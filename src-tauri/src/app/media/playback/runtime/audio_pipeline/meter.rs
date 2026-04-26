use crate::app::media::playback::events::MediaAudioMeterPayload;
use crate::app::media::playback::runtime::emit::emit_audio_meter_payloads;
use crate::app::media::state::AudioControls;
use rodio::Source;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::AppHandle;

const AUDIO_METER_BAND_COUNT: usize = 24;
const AUDIO_SPECTRUM_WINDOW: usize = 1024;
const AUDIO_METER_EMIT_HZ: u32 = 30;
const AUDIO_METER_POLL_MS: u64 = 16;

#[derive(Default)]
pub(crate) struct SharedAudioMeterState {
    sequence: u64,
    snapshot: Option<AudioMeterSnapshot>,
}

#[derive(Clone)]
struct AudioMeterSnapshot {
    sample_rate: u32,
    channels: u16,
    left_peak: f32,
    right_peak: f32,
    left_samples: Vec<f32>,
    right_samples: Vec<f32>,
}

pub(crate) type SharedAudioMeter = Arc<Mutex<SharedAudioMeterState>>;

pub(crate) fn create_shared_audio_meter() -> SharedAudioMeter {
    Arc::new(Mutex::new(SharedAudioMeterState::default()))
}

pub(crate) fn spawn_audio_meter_emitter(
    app: AppHandle,
    shared: SharedAudioMeter,
    stop_flag: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last_emitted_sequence = 0u64;
        while !stop_flag.load(Ordering::Relaxed) {
            let next_payload = shared
                .lock()
                .ok()
                .and_then(|state| {
                    if state.sequence > last_emitted_sequence {
                        Some((state.sequence, state.snapshot.clone()))
                    } else {
                        None
                    }
                });
            if let Some((sequence, Some(snapshot))) = next_payload {
                last_emitted_sequence = sequence;
                emit_audio_meter_payloads(&app, build_payload(snapshot));
            }
            thread::sleep(Duration::from_millis(AUDIO_METER_POLL_MS));
        }
    })
}

pub(crate) struct MeteredSource<S>
where
    S: Source<Item = f32>,
{
    inner: S,
    shared: SharedAudioMeter,
    controls: Arc<AudioControls>,
    accumulator: AudioMeterAccumulator,
    sample_index: usize,
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
            sample_index: 0,
        }
    }

    fn publish_snapshot(&self, snapshot: AudioMeterSnapshot) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.sequence = shared.sequence.saturating_add(1);
            shared.snapshot = Some(snapshot);
        }
    }
}

impl<S> Iterator for MeteredSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let processed = self.apply_live_controls(sample);
        if let Some(snapshot) = self.accumulator.push_sample(processed) {
            self.publish_snapshot(snapshot);
        }
        self.sample_index = self.sample_index.saturating_add(1);
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

impl<S> MeteredSource<S>
where
    S: Source<Item = f32>,
{
    fn apply_live_controls(&self, sample: f32) -> f32 {
        let channel_count = usize::from(self.inner.channels().get().max(1));
        let channel = self.sample_index % channel_count;
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
        let surround_gain = (left_gain + right_gain) * 0.5;
        let channel_gain = match channel {
            0 => left_gain,
            1 => right_gain,
            _ => surround_gain,
        };
        sample * global_gain * channel_gain
    }
}

struct AudioMeterAccumulator {
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
    fn new(sample_rate: u32, channels: usize) -> Self {
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

    fn push_sample(&mut self, sample: f32) -> Option<AudioMeterSnapshot> {
        self.pending_frame.push(sample);
        if self.pending_frame.len() < self.channels {
            return None;
        }
        let left = self.pending_frame.first().copied().unwrap_or(0.0);
        let right = if self.channels > 1 {
            self.pending_frame.get(1).copied().unwrap_or(left)
        } else {
            left
        };
        self.pending_frame.clear();
        Self::push_window_sample(&mut self.left_window, left);
        Self::push_window_sample(&mut self.right_window, right);
        self.interval_left_peak = self.interval_left_peak.max(left.abs());
        self.interval_right_peak = self.interval_right_peak.max(right.abs());
        self.frames_since_emit = self.frames_since_emit.saturating_add(1);
        if self.frames_since_emit < self.frames_per_emit {
            return None;
        }
        self.frames_since_emit = 0;
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
        Some(snapshot)
    }

    fn push_window_sample(window: &mut VecDeque<f32>, sample: f32) {
        if window.len() >= AUDIO_SPECTRUM_WINDOW {
            let _ = window.pop_front();
        }
        window.push_back(sample);
    }
}

pub(crate) fn compute_spectrum(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.len() < 32 || sample_rate == 0 {
        return vec![0.0; AUDIO_METER_BAND_COUNT];
    }
    let window_len = samples.len().min(AUDIO_SPECTRUM_WINDOW);
    let slice = &samples[samples.len() - window_len..];
    let nyquist = (sample_rate as f32) * 0.5;
    let min_freq = 32.0f32;
    let max_freq = nyquist.mul_add(0.92, 0.0).min(16_000.0).max(min_freq * 2.0);
    let ratio = max_freq / min_freq;
    (0..AUDIO_METER_BAND_COUNT)
        .map(|index| {
            let band_t = if AUDIO_METER_BAND_COUNT <= 1 {
                0.0
            } else {
                index as f32 / (AUDIO_METER_BAND_COUNT - 1) as f32
            };
            let frequency = min_freq * ratio.powf(band_t);
            let omega = (2.0 * PI * frequency) / (sample_rate as f32);
            let mut real = 0.0f32;
            let mut imag = 0.0f32;
            for (sample_index, sample) in slice.iter().enumerate() {
                let pos = sample_index as f32 / (window_len.saturating_sub(1).max(1) as f32);
                let window = 0.5 - 0.5 * (2.0 * PI * pos).cos();
                let phase = omega * sample_index as f32;
                real += sample * window * phase.cos();
                imag -= sample * window * phase.sin();
            }
            let magnitude = (real.mul_add(real, imag * imag)).sqrt() / (window_len as f32);
            let db = 20.0 * (magnitude + 1e-6).log10();
            ((db + 54.0) / 54.0).clamp(0.0, 1.0)
        })
        .collect()
}

fn build_payload(snapshot: AudioMeterSnapshot) -> MediaAudioMeterPayload {
    MediaAudioMeterPayload {
        sample_rate: snapshot.sample_rate,
        channels: snapshot.channels,
        left_peak: snapshot.left_peak,
        right_peak: snapshot.right_peak,
        left_spectrum: compute_spectrum(&snapshot.left_samples, snapshot.sample_rate),
        right_spectrum: compute_spectrum(&snapshot.right_samples, snapshot.sample_rate),
    }
}
