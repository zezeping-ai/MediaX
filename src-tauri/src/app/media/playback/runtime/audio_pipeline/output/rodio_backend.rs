use super::backend::AudioOutputBackend;
use super::types::{PlaybackHeadPosition, PlaybackHeadPrecision};
use crate::app::media::playback::runtime::audio_pipeline::meter::{
    create_shared_audio_meter, notify_meter_shutdown, spawn_audio_meter_emitter, MeteredSource,
    QueuedDurationTracker, SharedAudioMeter,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::state::AudioControls;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, Source};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

pub(super) struct RodioAudioOutputBackend {
    app: AppHandle,
    _stream: MixerDeviceSink,
    player: Player,
    controls: Arc<AudioControls>,
    meter_stop_flag: Arc<AtomicBool>,
    meter_thread: Option<JoinHandle<()>>,
    meter_shared: SharedAudioMeter,
    queued_duration: Arc<QueuedDurationTracker>,
    append_seq: AtomicU64,
    last_append_at_us: AtomicU64,
}

impl RodioAudioOutputBackend {
    pub(super) fn new(app: &AppHandle, controls: Arc<AudioControls>) -> Result<Self, String> {
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("open default audio output failed: {err}"))?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let meter_shared = create_shared_audio_meter();
        let queued_duration = Arc::new(QueuedDurationTracker::default());
        let meter_stop_flag = Arc::new(AtomicBool::new(false));
        let meter_thread =
            spawn_audio_meter_emitter(app.clone(), meter_shared.clone(), meter_stop_flag.clone());
        let backend = Self {
            app: app.clone(),
            _stream: stream,
            player,
            controls,
            meter_stop_flag,
            meter_thread: Some(meter_thread),
            meter_shared,
            queued_duration,
            append_seq: AtomicU64::new(0),
            last_append_at_us: AtomicU64::new(now_unix_micros()),
        };
        backend.player.play();
        Ok(backend)
    }
}

impl AudioOutputBackend for RodioAudioOutputBackend {
    fn backend_name(&self) -> &'static str {
        "rodio"
    }

    fn playback_head_precision(&self) -> PlaybackHeadPrecision {
        PlaybackHeadPrecision::Estimated
    }

    fn preferred_sample_rate(&self) -> Option<u32> {
        None
    }

    fn queue_depth(&self) -> usize {
        self.player.len()
    }

    fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    fn resume(&self) {
        self.player.play();
    }

    fn pause_and_clear_queue(&self) {
        emit_debug(
            &self.app,
            "audio_output_pause_clear",
            format!(
                "pause+clear queue_depth={} queued_ms={:.2}",
                self.player.len(),
                self.queued_duration.queued_seconds() * 1000.0,
            ),
        );
        self.player.clear();
        self.queued_duration.clear();
        self.player.pause();
    }

    fn clear_queue(&self) {
        emit_debug(
            &self.app,
            "audio_output_clear_queue",
            format!(
                "clear queue_depth={} queued_ms={:.2}",
                self.player.len(),
                self.queued_duration.queued_seconds() * 1000.0,
            ),
        );
        self.player.clear();
        self.queued_duration.clear();
        self.player.play();
    }

    fn queued_duration_seconds(&self) -> f64 {
        self.queued_duration.queued_seconds()
    }

    fn observed_playback_head_position(
        &self,
        estimated_extra_latency_seconds: f64,
    ) -> Option<PlaybackHeadPosition> {
        self.queued_duration
            .playback_head_seconds(estimated_extra_latency_seconds)
            .map(PlaybackHeadPosition::estimated)
    }

    fn append_pcm_f32_owned(
        &self,
        sample_rate: u32,
        channels: u16,
        pcm: Vec<f32>,
        media_start_seconds: Option<f64>,
        media_duration_seconds: f64,
    ) {
        let Some(channels) = NonZeroU16::new(channels) else {
            return;
        };
        let Some(sample_rate) = NonZeroU32::new(sample_rate) else {
            return;
        };
        if pcm.is_empty() {
            return;
        }
        let frame_count = pcm.len() / usize::from(channels.get());
        let duration_micros =
            ((frame_count as u128) * 1_000_000u128 / u128::from(sample_rate.get())) as u64;
        let queued_block_id = self.queued_duration.push_block(
            duration_micros,
            media_start_seconds,
            media_duration_seconds,
        );
        let now_us = now_unix_micros();
        let prev_us = self.last_append_at_us.swap(now_us, Ordering::Relaxed);
        let append_gap_ms = if prev_us == 0 || now_us <= prev_us {
            0.0
        } else {
            (now_us - prev_us) as f64 / 1000.0
        };
        let seq = self.append_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let block_ms = duration_micros as f64 / 1000.0;
        let queue_depth_before_append = self.player.len();
        let queued_ms_before_append = self.queued_duration.queued_seconds() * 1000.0;
        if seq <= 16 || seq % 120 == 0 || append_gap_ms >= 80.0 {
            emit_debug(
                &self.app,
                "audio_output_append",
                format!(
                    "seq={} queue_depth_before={} queued_ms_before={:.2} block_ms={:.2} append_gap_ms={:.2} frames={} rate={} channels={}",
                    seq,
                    queue_depth_before_append,
                    queued_ms_before_append,
                    block_ms,
                    append_gap_ms,
                    frame_count,
                    sample_rate.get(),
                    channels.get(),
                ),
            );
        }
        let source = OwnedPcmSource::new(channels, sample_rate, pcm);
        self.player.append(MeteredSource::new(
            source,
            self.meter_shared.clone(),
            self.controls.clone(),
            self.queued_duration.clone(),
            queued_block_id,
        ));
    }
}

fn now_unix_micros() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_micros() as u64)
        .unwrap_or(0)
}

impl Drop for RodioAudioOutputBackend {
    fn drop(&mut self) {
        self.meter_stop_flag.store(true, Ordering::Relaxed);
        notify_meter_shutdown(&self.meter_shared);
        if let Some(thread) = self.meter_thread.take() {
            let _ = thread.join();
        }
    }
}

struct OwnedPcmSource {
    data: Vec<f32>,
    position: usize,
    channels: ChannelCount,
    sample_rate: SampleRate,
    duration: Duration,
}

impl OwnedPcmSource {
    fn new(channels: ChannelCount, sample_rate: SampleRate, data: Vec<f32>) -> Self {
        let duration_micros = ((data.len() as u128) * 1_000_000u128)
            / u128::from(sample_rate.get())
            / u128::from(channels.get());
        Self {
            data,
            position: 0,
            channels,
            sample_rate,
            duration: Duration::from_micros(duration_micros as u64),
        }
    }
}

impl Iterator for OwnedPcmSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.data.get(self.position).copied()?;
        self.position += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.data.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for OwnedPcmSource {}

impl Source for OwnedPcmSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.data.len().saturating_sub(self.position))
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let curr_channel = self.position % self.channels.get() as usize;
        let new_pos =
            pos.as_secs_f64() * self.sample_rate.get() as f64 * self.channels.get() as f64;
        let new_pos = (new_pos as usize).min(self.data.len());
        let new_pos = new_pos.next_multiple_of(self.channels.get() as usize);
        self.position = new_pos.saturating_sub(curr_channel);
        Ok(())
    }
}
