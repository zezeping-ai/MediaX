use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;

use crate::app::media::model::{MediaLyricLine, PlaybackMediaKind};
use crate::app::media::playback::render::renderer::VideoFrame;

pub(crate) struct VideoDecodeContext {
    pub(crate) input_ctx: format::context::Input,
    pub(crate) video_stream_index: Option<usize>,
    pub(crate) video_time_base: Option<ffmpeg::Rational>,
    pub(crate) decoder: Option<ffmpeg::decoder::Video>,
    pub(crate) fps_value: f64,
    pub(crate) duration_seconds: f64,
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
    pub(crate) media_kind: PlaybackMediaKind,
    pub(crate) has_cover_art: bool,
    pub(crate) cover_frame: Option<VideoFrame>,
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) lyrics: Vec<MediaLyricLine>,
    pub(crate) hw_decode_active: bool,
    pub(crate) hw_decode_backend: Option<String>,
    pub(crate) hw_decode_error: Option<String>,
    pub(crate) hw_decode_decision: String,
}

pub(crate) struct HwDecodeStatus {
    pub(crate) active: bool,
    pub(crate) backend: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) decision: String,
}
