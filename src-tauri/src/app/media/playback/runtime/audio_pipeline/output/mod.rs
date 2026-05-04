mod backend;
mod cpal_backend;
mod rodio_backend;
mod types;

use self::backend::AudioOutputBackend;
use self::cpal_backend::CpalAudioOutputBackend;
use self::rodio_backend::RodioAudioOutputBackend;
pub(crate) use self::types::{PlaybackHeadPosition, PlaybackHeadPrecision};
use crate::app::media::state::AudioControls;
use std::sync::Arc;
use tauri::AppHandle;

pub(crate) struct AudioOutput {
    backend: Box<dyn AudioOutputBackend>,
}

impl AudioOutput {
    pub fn new(app: &AppHandle, controls: Arc<AudioControls>) -> Result<Self, String> {
        let backend = select_backend(app, controls)?;
        Ok(Self { backend })
    }

    pub fn backend_name(&self) -> &'static str {
        self.backend.backend_name()
    }

    pub fn playback_head_precision(&self) -> PlaybackHeadPrecision {
        self.backend.playback_head_precision()
    }

    pub fn preferred_sample_rate(&self) -> Option<u32> {
        self.backend.preferred_sample_rate()
    }

    pub fn queue_depth(&self) -> usize {
        self.backend.queue_depth()
    }

    pub fn is_paused(&self) -> bool {
        self.backend.is_paused()
    }

    pub fn resume(&self) {
        self.backend.resume();
    }

    pub fn pause_and_clear_queue(&self) {
        self.backend.pause_and_clear_queue();
    }

    pub fn clear_queue(&self) {
        self.backend.clear_queue();
    }

    pub fn queued_duration_seconds(&self) -> f64 {
        self.backend.queued_duration_seconds()
    }

    pub fn observed_playback_head_position(
        &self,
        estimated_extra_latency_seconds: f64,
    ) -> Option<PlaybackHeadPosition> {
        self.backend
            .observed_playback_head_position(estimated_extra_latency_seconds)
    }

    pub fn append_pcm_f32_owned(
        &self,
        sample_rate: u32,
        channels: u16,
        pcm: Vec<f32>,
        media_start_seconds: Option<f64>,
        media_duration_seconds: f64,
    ) {
        self.backend.append_pcm_f32_owned(
            sample_rate,
            channels,
            pcm,
            media_start_seconds,
            media_duration_seconds,
        );
    }
}

fn select_backend(
    app: &AppHandle,
    controls: Arc<AudioControls>,
) -> Result<Box<dyn AudioOutputBackend>, String> {
    let preferred_backend = std::env::var("MEDIAX_AUDIO_BACKEND").ok();
    if preferred_backend
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("cpal"))
    {
        eprintln!("mediax audio backend: trying cpal");
        match CpalAudioOutputBackend::new(app, controls.clone()) {
            Ok(backend) => {
                eprintln!("mediax audio backend: using cpal");
                return Ok(Box::new(backend));
            }
            Err(err) => {
                eprintln!("cpal backend init failed, falling back to rodio: {err}");
            }
        }
    }
    let backend = RodioAudioOutputBackend::new(app, controls)?;
    eprintln!("mediax audio backend: using rodio");
    Ok(Box::new(backend))
}
