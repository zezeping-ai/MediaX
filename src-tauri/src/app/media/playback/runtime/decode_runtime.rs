use super::audio_pipeline::AudioPipeline;
use super::session::DecodeLoopState;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;

pub(super) struct DecodeRuntime {
    pub video_ctx: VideoDecodeContext,
    pub scaler: Option<ScalingContext>,
    pub audio_pipeline: Option<AudioPipeline>,
    pub loop_state: DecodeLoopState,
    pub should_tail_eof: bool,
}

impl DecodeRuntime {
    pub fn has_video_stream(&self) -> bool {
        self.video_ctx.video_stream_index.is_some()
    }

    pub fn audio_stream_index(&self) -> Option<usize> {
        self.audio_pipeline.as_ref().map(|audio| audio.stream_index)
    }

    pub fn is_video_packet(&self, packet_stream_index: usize) -> bool {
        self.video_ctx
            .video_stream_index
            .is_some_and(|index| packet_stream_index == index)
    }
}
