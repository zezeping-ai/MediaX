use super::types::AudioOutput;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::state::{AudioControls, TimingControls};
use rodio::{buffer::SamplesBuffer, DeviceSinkBuilder, Player};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::Arc;

impl AudioOutput {
    pub fn new(
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

    pub fn apply_controls(&self) {
        let volume = if self.controls.muted() {
            0.0
        } else {
            self.controls.volume()
        };
        self.player.set_volume(volume);
        self.player
            .set_speed(clamp_playback_rate(self.timing_controls.playback_rate()));
    }

    pub fn append_pcm_i16(&self, sample_rate: u32, channels: u16, pcm: &[i16]) {
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
