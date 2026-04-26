use crate::app::media::playback::render::renderer::RendererState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use std::time::Instant;

pub(super) struct PreviewFrameContext<'a> {
    pub renderer: &'a RendererState,
    pub decoder: &'a mut ffmpeg::decoder::Video,
    pub scaler: &'a mut Option<ScalingContext>,
    pub output_width: u32,
    pub output_height: u32,
    pub video_time_base: ffmpeg::Rational,
    pub target_seconds: f64,
    pub seek_applied: bool,
    pub deadline: Instant,
}

pub(super) struct DecodePreviewFrameContext<'a> {
    pub decoder: &'a mut ffmpeg::decoder::Video,
    pub scaler: &'a mut Option<ScalingContext>,
    pub output_width: u32,
    pub output_height: u32,
    pub video_time_base: ffmpeg::Rational,
    pub target_seconds: f64,
    pub seek_applied: bool,
    pub deadline: Instant,
}
