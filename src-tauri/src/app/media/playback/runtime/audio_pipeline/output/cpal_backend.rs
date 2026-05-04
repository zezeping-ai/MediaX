use super::backend::AudioOutputBackend;
use super::types::{PlaybackHeadPosition, PlaybackHeadPrecision};
use crate::app::media::playback::dto::PlaybackChannelRouting;
use crate::app::media::playback::runtime::audio_pipeline::meter::{
    create_shared_audio_meter, notify_meter_shutdown, publish_snapshot, spawn_audio_meter_emitter,
    AudioMeterAccumulator, QueuedDurationTracker, SharedAudioMeter,
};
use crate::app::media::state::AudioControls;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample, Stream, StreamConfig};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(super) struct CpalAudioOutputBackend {
    stream: Stream,
    stream_sample_rate: u32,
    paused: Arc<AtomicBool>,
    meter_stop_flag: Arc<AtomicBool>,
    meter_thread: Option<JoinHandle<()>>,
    meter_shared: SharedAudioMeter,
    queued_duration: Arc<QueuedDurationTracker>,
    shared: Arc<Mutex<CpalSharedState>>,
}

struct CpalSharedState {
    controls: Arc<AudioControls>,
    blocks: VecDeque<CpalQueuedBlock>,
    meter: AudioMeterAccumulator,
    playback_anchor: Option<PlaybackAnchor>,
    underrun_count: u64,
    debug_callback_logs_remaining: u8,
    append_debug_logs_remaining: u8,
    underrun_debug_logs_remaining: u8,
}

struct CpalQueuedBlock {
    id: u64,
    pcm: Vec<f32>,
    source_channels: u16,
    media_start_seconds: Option<f64>,
    media_duration_seconds: f64,
    position_samples: usize,
}

#[derive(Clone, Copy)]
struct PlaybackAnchor {
    playback_start_instant: Instant,
    playback_end_instant: Instant,
    media_start_seconds: f64,
    media_end_seconds: f64,
}

impl CpalAudioOutputBackend {
    pub(super) fn new(app: &AppHandle, controls: Arc<AudioControls>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "cpal default output device not found".to_string())?;
        let supported_config = device
            .default_output_config()
            .map_err(|err| format!("cpal default output config failed: {err}"))?;
        let stream_config: StreamConfig = supported_config.config();
        let stream_channels = stream_config.channels;
        let stream_sample_rate = stream_config.sample_rate;
        let meter_shared = create_shared_audio_meter();
        let queued_duration = Arc::new(QueuedDurationTracker::default());
        let meter_stop_flag = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(true));
        let meter_thread =
            spawn_audio_meter_emitter(app.clone(), meter_shared.clone(), meter_stop_flag.clone());
        let shared = Arc::new(Mutex::new(CpalSharedState {
            controls,
            blocks: VecDeque::new(),
            meter: AudioMeterAccumulator::new(
                stream_sample_rate,
                usize::from(stream_channels.max(1)),
            ),
            playback_anchor: None,
            underrun_count: 0,
            debug_callback_logs_remaining: 8,
            append_debug_logs_remaining: 24,
            underrun_debug_logs_remaining: 12,
        }));
        let error_backend_name = "cpal";
        let err_fn = move |err| {
            eprintln!("{error_backend_name} stream error: {err}");
        };
        let stream = match supported_config.sample_format() {
            cpal::SampleFormat::F32 => build_output_stream::<f32>(
                &device,
                &stream_config,
                stream_sample_rate,
                paused.clone(),
                shared.clone(),
                meter_shared.clone(),
                queued_duration.clone(),
                err_fn,
            ),
            cpal::SampleFormat::I16 => build_output_stream::<i16>(
                &device,
                &stream_config,
                stream_sample_rate,
                paused.clone(),
                shared.clone(),
                meter_shared.clone(),
                queued_duration.clone(),
                err_fn,
            ),
            cpal::SampleFormat::U16 => build_output_stream::<u16>(
                &device,
                &stream_config,
                stream_sample_rate,
                paused.clone(),
                shared.clone(),
                meter_shared.clone(),
                queued_duration.clone(),
                err_fn,
            ),
            other => Err(format!("unsupported cpal sample format: {other:?}")),
        }?;
        stream
            .pause()
            .map_err(|err| format!("cpal stream initial pause failed: {err}"))?;
        Ok(Self {
            stream,
            stream_sample_rate,
            paused,
            meter_stop_flag,
            meter_thread: Some(meter_thread),
            meter_shared,
            queued_duration,
            shared,
        })
    }
}

impl AudioOutputBackend for CpalAudioOutputBackend {
    fn backend_name(&self) -> &'static str {
        "cpal"
    }

    fn playback_head_precision(&self) -> PlaybackHeadPrecision {
        PlaybackHeadPrecision::Measured
    }

    fn preferred_sample_rate(&self) -> Option<u32> {
        Some(self.stream_sample_rate)
    }

    fn queue_depth(&self) -> usize {
        self.shared
            .lock()
            .ok()
            .map(|state| state.blocks.len())
            .unwrap_or(0)
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    fn resume(&self) {
        if self
            .paused
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            eprintln!("cpal resume debug: requesting stream.play()");
            if self.stream.play().is_err() {
                self.paused.store(true, Ordering::Relaxed);
                eprintln!("cpal resume debug: stream.play() failed");
            } else {
                eprintln!("cpal resume debug: stream.play() ok");
            }
        }
    }

    fn pause_and_clear_queue(&self) {
        self.clear_queue();
        self.paused.store(true, Ordering::Relaxed);
        let _ = self.stream.pause();
    }

    fn clear_queue(&self) {
        if let Ok(mut state) = self.shared.lock() {
            state.blocks.clear();
            state.playback_anchor = None;
        }
        self.queued_duration.clear();
    }

    fn queued_duration_seconds(&self) -> f64 {
        self.queued_duration.queued_seconds()
    }

    fn observed_playback_head_position(
        &self,
        _estimated_extra_latency_seconds: f64,
    ) -> Option<PlaybackHeadPosition> {
        let anchor = self
            .shared
            .lock()
            .ok()
            .and_then(|state| state.playback_anchor)?;
        let now = Instant::now();
        let window_duration_seconds = anchor
            .playback_end_instant
            .saturating_duration_since(anchor.playback_start_instant)
            .as_secs_f64();
        let progress = if now <= anchor.playback_start_instant {
            0.0
        } else if now >= anchor.playback_end_instant || window_duration_seconds <= f64::EPSILON {
            1.0
        } else {
            now.saturating_duration_since(anchor.playback_start_instant)
                .as_secs_f64()
                / window_duration_seconds
        }
        .clamp(0.0, 1.0);
        let media_seconds = anchor.media_start_seconds
            + (anchor.media_end_seconds - anchor.media_start_seconds).max(0.0) * progress;
        Some(PlaybackHeadPosition {
            // CPAL callback timestamps already point at the device playback timeline, so this
            // measured head should not be shifted again by queue-latency estimates. Doing both
            // systematically drags the observed audio clock behind audible output.
            seconds: media_seconds.max(0.0),
            precision: PlaybackHeadPrecision::Measured,
        })
    }

    fn append_pcm_f32_owned(
        &self,
        sample_rate: u32,
        channels: u16,
        pcm: Vec<f32>,
        media_start_seconds: Option<f64>,
        media_duration_seconds: f64,
    ) {
        if pcm.is_empty() || channels == 0 {
            return;
        }
        let frame_count = pcm.len() / usize::from(channels);
        // CPAL streams consume frames at the negotiated device rate. Keep queue timing tied to
        // that cadence even if an upstream mismatch slips through, while the pipeline is expected
        // to resample into `stream_sample_rate` before data reaches this backend.
        debug_assert!(
            sample_rate == self.stream_sample_rate,
            "cpal backend expects upstream PCM to be resampled into the negotiated device rate",
        );
        let duration_micros =
            ((frame_count as u128) * 1_000_000u128 / u128::from(self.stream_sample_rate)) as u64;
        let queued_block_id = self.queued_duration.push_block(
            duration_micros,
            media_start_seconds,
            media_duration_seconds,
        );
        if let Ok(mut state) = self.shared.lock() {
            state.blocks.push_back(CpalQueuedBlock {
                id: queued_block_id,
                pcm,
                source_channels: channels,
                media_start_seconds,
                media_duration_seconds,
                position_samples: 0,
            });
            if cfg!(debug_assertions) && state.append_debug_logs_remaining > 0 {
                eprintln!(
                    "cpal append debug: block_id={} frames={} queue_blocks={} paused={}",
                    queued_block_id,
                    frame_count,
                    state.blocks.len(),
                    self.paused.load(Ordering::Relaxed),
                );
                state.append_debug_logs_remaining =
                    state.append_debug_logs_remaining.saturating_sub(1);
            }
        }
    }
}

impl Drop for CpalAudioOutputBackend {
    fn drop(&mut self) {
        self.meter_stop_flag.store(true, Ordering::Relaxed);
        notify_meter_shutdown(&self.meter_shared);
        if let Some(thread) = self.meter_thread.take() {
            let _ = thread.join();
        }
    }
}

fn build_output_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    stream_sample_rate: u32,
    paused: Arc<AtomicBool>,
    shared: Arc<Mutex<CpalSharedState>>,
    meter_shared: SharedAudioMeter,
    queued_duration: Arc<QueuedDurationTracker>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let output_channels = usize::from(config.channels.max(1));
    device
        .build_output_stream(
            config,
            move |data: &mut [T], info| {
                write_output_data::<T>(
                    data,
                    output_channels,
                    stream_sample_rate,
                    &paused,
                    &shared,
                    &meter_shared,
                    &queued_duration,
                    info,
                );
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("cpal build output stream failed: {err}"))
}

fn write_output_data<T>(
    data: &mut [T],
    output_channels: usize,
    stream_sample_rate: u32,
    paused: &Arc<AtomicBool>,
    shared: &Arc<Mutex<CpalSharedState>>,
    meter_shared: &SharedAudioMeter,
    queued_duration: &Arc<QueuedDurationTracker>,
    info: &cpal::OutputCallbackInfo,
) where
    T: SizedSample + FromSample<f32>,
{
    for sample in data.iter_mut() {
        *sample = T::from_sample(0.0);
    }
    if paused.load(Ordering::Relaxed) {
        return;
    }
    let mut state = match shared.lock() {
        Ok(state) => state,
        Err(_) => return,
    };
    let mut first_media_seconds = None;
    let mut last_media_end_seconds = None;
    let playback_window_start = measured_playback_anchor_instant(info).unwrap_or_else(Instant::now);
    let callback_frames = data.len() / output_channels.max(1);
    let mut silent_frames = 0usize;
    for frame_samples in data.chunks_mut(output_channels) {
        let Some((media_seconds, media_end_seconds)) =
            write_next_output_frame::<T>(&mut state, frame_samples, queued_duration, meter_shared)
        else {
            silent_frames = silent_frames.saturating_add(1);
            continue;
        };
        if first_media_seconds.is_none() {
            first_media_seconds = media_seconds;
        }
        last_media_end_seconds = media_end_seconds;
    }
    if let (Some(media_start_seconds), Some(media_end_seconds)) =
        (first_media_seconds, last_media_end_seconds)
    {
        let callback_duration =
            Duration::from_secs_f64(callback_frames as f64 / stream_sample_rate.max(1) as f64);
        let playback_lead_ms = info
            .timestamp()
            .playback
            .duration_since(&info.timestamp().callback)
            .map(|duration| duration.as_secs_f64() * 1000.0);
        state.playback_anchor = Some(PlaybackAnchor {
            playback_start_instant: playback_window_start,
            playback_end_instant: playback_window_start + callback_duration,
            media_start_seconds,
            media_end_seconds,
        });
        if cfg!(debug_assertions) && state.debug_callback_logs_remaining > 0 {
            let media_span_ms = (media_end_seconds - media_start_seconds).max(0.0) * 1000.0;
            eprintln!(
                "cpal callback debug: frames={} rate={}Hz wall_ms={:.3} playback_lead_ms={:.3?} media_start={:.6} media_end={:.6} media_span_ms={:.3}",
                callback_frames,
                stream_sample_rate,
                callback_duration.as_secs_f64() * 1000.0,
                playback_lead_ms,
                media_start_seconds,
                media_end_seconds,
                media_span_ms,
            );
            state.debug_callback_logs_remaining =
                state.debug_callback_logs_remaining.saturating_sub(1);
        }
    }
    if silent_frames > 0 && state.underrun_debug_logs_remaining > 0 {
        state.underrun_count = state.underrun_count.saturating_add(silent_frames as u64);
        let silent_ms = (silent_frames as f64 / stream_sample_rate.max(1) as f64) * 1000.0;
        let queued_blocks = state.blocks.len();
        let queued_ms = queued_duration.queued_seconds() * 1000.0;
        eprintln!(
            "cpal underrun debug: callback_frames={} silent_frames={} silent_ms={:.3} queued_blocks={} queued_ms={:.3}",
            callback_frames,
            silent_frames,
            silent_ms,
            queued_blocks,
            queued_ms,
        );
        state.underrun_debug_logs_remaining = state.underrun_debug_logs_remaining.saturating_sub(1);
    } else if silent_frames > 0 {
        state.underrun_count = state.underrun_count.saturating_add(silent_frames as u64);
    }
}

fn measured_playback_anchor_instant(info: &cpal::OutputCallbackInfo) -> Option<Instant> {
    let timestamp = info.timestamp();
    let lead = timestamp.playback.duration_since(&timestamp.callback)?;
    Some(Instant::now() + lead)
}

fn write_next_output_frame<T>(
    state: &mut CpalSharedState,
    output_frame: &mut [T],
    queued_duration: &Arc<QueuedDurationTracker>,
    meter_shared: &SharedAudioMeter,
) -> Option<(Option<f64>, Option<f64>)>
where
    T: SizedSample + FromSample<f32>,
{
    loop {
        enum FrameStep {
            SkipBlock {
                finished_id: u64,
            },
            EmitFrame {
                finished_id: u64,
                media_seconds: Option<f64>,
                media_end_seconds: Option<f64>,
                left: f32,
                right: f32,
                finished_block: bool,
            },
        }
        let step = {
            let block = state.blocks.front_mut()?;
            let source_channels = usize::from(block.source_channels.max(1));
            if block.position_samples == 0 {
                queued_duration.mark_block_started(block.id);
            }
            if block.position_samples + source_channels > block.pcm.len() {
                FrameStep::SkipBlock {
                    finished_id: block.id,
                }
            } else {
                let media_seconds = block_media_seconds(block);
                let media_end_seconds = block_media_end_seconds(block);
                let left = block.pcm[block.position_samples];
                let right = if source_channels > 1 {
                    block.pcm[block.position_samples + 1]
                } else {
                    left
                };
                block.position_samples += source_channels;
                FrameStep::EmitFrame {
                    finished_id: block.id,
                    media_seconds,
                    media_end_seconds,
                    left,
                    right,
                    finished_block: block.position_samples >= block.pcm.len(),
                }
            }
        };
        match step {
            FrameStep::SkipBlock { finished_id } => {
                state.blocks.pop_front();
                queued_duration.finish_block(finished_id);
            }
            FrameStep::EmitFrame {
                finished_id,
                media_seconds,
                media_end_seconds,
                left,
                right,
                finished_block,
            } => {
                if finished_block {
                    state.blocks.pop_front();
                    queued_duration.finish_block(finished_id);
                }
                write_output_frame(
                    output_frame,
                    left,
                    right,
                    &state.controls,
                    &mut state.meter,
                    meter_shared,
                );
                return Some((media_seconds, media_end_seconds));
            }
        }
    }
}

fn block_media_seconds(block: &CpalQueuedBlock) -> Option<f64> {
    let media_start_seconds = block
        .media_start_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)?;
    let total_samples = block.pcm.len();
    if total_samples == 0 {
        return Some(media_start_seconds);
    }
    let progress = (block.position_samples as f64 / total_samples as f64).clamp(0.0, 1.0);
    Some((media_start_seconds + block.media_duration_seconds.max(0.0) * progress).max(0.0))
}

fn block_media_end_seconds(block: &CpalQueuedBlock) -> Option<f64> {
    let media_start_seconds = block
        .media_start_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)?;
    let total_frames = block.pcm.len() / usize::from(block.source_channels.max(1));
    if total_frames == 0 {
        return Some(media_start_seconds);
    }
    let emitted_frames = block.position_samples / usize::from(block.source_channels.max(1));
    let progress = (emitted_frames as f64 / total_frames as f64).clamp(0.0, 1.0);
    Some((media_start_seconds + block.media_duration_seconds.max(0.0) * progress).max(0.0))
}

fn apply_live_controls(controls: &AudioControls, left: &mut f32, right: &mut f32) {
    let global_gain = if controls.muted() {
        0.0
    } else {
        controls.volume()
    };
    let left_gain = if controls.left_muted() {
        0.0
    } else {
        controls.left_volume()
    };
    let right_gain = if controls.right_muted() {
        0.0
    } else {
        controls.right_volume()
    };
    match controls.channel_routing() {
        PlaybackChannelRouting::Stereo => {}
        PlaybackChannelRouting::LeftToBoth => *right = *left,
        PlaybackChannelRouting::RightToBoth => *left = *right,
    }
    *left *= global_gain * left_gain;
    *right *= global_gain * right_gain;
}

fn publish_meter_frame(
    accumulator: &mut AudioMeterAccumulator,
    shared: &SharedAudioMeter,
    left: f32,
    right: f32,
) {
    for sample in [left, right] {
        if let Some(snapshot) = accumulator.push_sample(sample) {
            publish_snapshot(shared, snapshot);
        }
    }
}

fn write_output_frame<T>(
    output_frame: &mut [T],
    source_left: f32,
    source_right: f32,
    controls: &AudioControls,
    meter: &mut AudioMeterAccumulator,
    meter_shared: &SharedAudioMeter,
) where
    T: SizedSample + FromSample<f32>,
{
    if output_frame.is_empty() {
        return;
    }
    let mut left = source_left;
    let mut right = source_right;
    apply_live_controls(controls, &mut left, &mut right);
    publish_meter_frame(meter, meter_shared, left, right);
    for (channel_index, sample) in output_frame.iter_mut().enumerate() {
        let value = match channel_index {
            0 => left,
            1 => right,
            _ => (left + right) * 0.5,
        };
        *sample = T::from_sample(value);
    }
}
