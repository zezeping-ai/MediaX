use super::meter::{
    create_shared_audio_meter, notify_meter_shutdown, spawn_audio_meter_emitter, MeteredSource,
    QueuedDurationTracker, SharedAudioMeter,
};
use crate::app::media::state::AudioControls;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, Source};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::AppHandle;

pub(crate) struct AudioOutput {
    pub _stream: MixerDeviceSink,
    pub player: Player,
    pub controls: Arc<AudioControls>,
    pub meter_stop_flag: Arc<AtomicBool>,
    pub meter_thread: Option<JoinHandle<()>>,
    pub meter_shared: SharedAudioMeter,
    pub queued_duration: Arc<QueuedDurationTracker>,
}

impl AudioOutput {
    pub fn new(app: &AppHandle, controls: Arc<AudioControls>) -> Result<Self, String> {
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("open default audio output failed: {err}"))?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let meter_shared = create_shared_audio_meter();
        let queued_duration = Arc::new(QueuedDurationTracker::default());
        let meter_stop_flag = Arc::new(AtomicBool::new(false));
        let meter_thread =
            spawn_audio_meter_emitter(app.clone(), meter_shared.clone(), meter_stop_flag.clone());
        let output = Self {
            _stream: stream,
            player,
            controls,
            meter_stop_flag,
            meter_thread: Some(meter_thread),
            meter_shared,
            queued_duration,
        };
        output.player.play();
        Ok(output)
    }

    pub fn queue_depth(&self) -> usize {
        self.player.len()
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn pause_and_clear_queue(&self) {
        self.player.clear();
        self.queued_duration.clear();
        self.player.pause();
    }

    pub fn clear_queue(&self) {
        self.player.clear();
        self.queued_duration.clear();
        self.player.play();
    }

    pub fn queued_duration_seconds(&self) -> f64 {
        self.queued_duration.queued_seconds()
    }

    pub fn append_pcm_f32_owned(&self, sample_rate: u32, channels: u16, pcm: Vec<f32>) {
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
        self.queued_duration.push_block(duration_micros);
        let source = OwnedPcmSource::new(channels, sample_rate, pcm);
        self.player.append(MeteredSource::new(
            source,
            self.meter_shared.clone(),
            self.controls.clone(),
            self.queued_duration.clone(),
        ));
    }
}

impl Drop for AudioOutput {
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
        let new_pos = pos.as_secs_f64()
            * self.sample_rate.get() as f64
            * self.channels.get() as f64;
        let new_pos = (new_pos as usize).min(self.data.len());
        let new_pos = new_pos.next_multiple_of(self.channels.get() as usize);
        self.position = new_pos.saturating_sub(curr_channel);
        Ok(())
    }
}
