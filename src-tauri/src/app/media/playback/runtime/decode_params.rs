use crate::app::media::playback::dto::HardwareDecodeMode;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::{AudioControls, TimingControls};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::AppHandle;

#[derive(Clone, Copy)]
pub(crate) struct DecodeRequest<'a> {
    pub source: &'a str,
    pub stream_generation: u32,
    pub hw_mode_override: HardwareDecodeMode,
    pub software_fallback_reason: Option<&'a str>,
    pub force_audio_only: bool,
}

impl<'a> DecodeRequest<'a> {
    pub fn software_fallback(&self, reason: &'a str) -> Self {
        Self {
            source: self.source,
            stream_generation: self.stream_generation,
            hw_mode_override: HardwareDecodeMode::Off,
            software_fallback_reason: Some(reason),
            force_audio_only: self.force_audio_only,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct DecodeDependencies<'a> {
    pub app: &'a AppHandle,
    pub renderer: &'a RendererState,
    pub stop_flag: &'a Arc<AtomicBool>,
    pub audio_controls: &'a Arc<AudioControls>,
    pub timing_controls: &'a Arc<TimingControls>,
}
