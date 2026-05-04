use super::types::{PlaybackHeadPosition, PlaybackHeadPrecision};

pub(crate) trait AudioOutputBackend {
    fn backend_name(&self) -> &'static str;

    fn playback_head_precision(&self) -> PlaybackHeadPrecision;

    fn preferred_sample_rate(&self) -> Option<u32>;

    fn queue_depth(&self) -> usize;

    fn is_paused(&self) -> bool;

    fn resume(&self);

    fn pause_and_clear_queue(&self);

    fn clear_queue(&self);

    fn queued_duration_seconds(&self) -> f64;

    fn observed_playback_head_position(
        &self,
        estimated_extra_latency_seconds: f64,
    ) -> Option<PlaybackHeadPosition>;

    fn append_pcm_f32_owned(
        &self,
        sample_rate: u32,
        channels: u16,
        pcm: Vec<f32>,
        media_start_seconds: Option<f64>,
        media_duration_seconds: f64,
    );
}
