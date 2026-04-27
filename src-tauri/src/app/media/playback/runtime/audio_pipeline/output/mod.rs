use super::meter::{
    create_shared_audio_meter, spawn_audio_meter_emitter, MeteredSource, SharedAudioMeter,
};
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::state::{AudioControls, TimingControls};
use rodio::{buffer::SamplesBuffer, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use tauri::AppHandle;

pub(crate) struct AudioOutput {
    pub _stream: MixerDeviceSink,
    pub player: Player,
    pub controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
    pub meter_stop_flag: Arc<AtomicBool>,
    pub meter_thread: Option<JoinHandle<()>>,
    pub meter_shared: SharedAudioMeter,
}

impl AudioOutput {
    pub fn new(
        app: &AppHandle,
        controls: Arc<AudioControls>,
        timing_controls: Arc<TimingControls>,
    ) -> Result<Self, String> {
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("open default audio output failed: {err}"))?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let meter_shared = create_shared_audio_meter();
        let meter_stop_flag = Arc::new(AtomicBool::new(false));
        let meter_thread =
            spawn_audio_meter_emitter(app.clone(), meter_shared.clone(), meter_stop_flag.clone());
        let output = Self {
            _stream: stream,
            player,
            controls,
            timing_controls,
            meter_stop_flag,
            meter_thread: Some(meter_thread),
            meter_shared,
        };
        output.player.play();
        output.apply_controls();
        Ok(output)
    }

    pub fn apply_controls(&self) {
        self.player
            .set_speed(clamp_playback_rate(self.timing_controls.playback_rate()));
    }

    pub fn queue_depth(&self) -> usize {
        self.player.len()
    }

    pub fn set_speed(&self, speed: f32) {
        self.player.set_speed(speed);
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn clear_queue(&self) {
        self.player.clear();
        self.player.play();
    }

    pub fn append_pcm_f32(&self, sample_rate: u32, channels: u16, pcm: &[f32]) {
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
        let source = SamplesBuffer::new(channels, sample_rate, pcm.to_vec());
        self.player.append(MeteredSource::new(
            source,
            self.meter_shared.clone(),
            self.controls.clone(),
        ));
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.meter_stop_flag.store(true, Ordering::Relaxed);
        if let Some(thread) = self.meter_thread.take() {
            let _ = thread.join();
        }
    }
}
