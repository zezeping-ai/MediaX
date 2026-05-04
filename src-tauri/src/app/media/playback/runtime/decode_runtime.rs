use super::audio_pipeline::AudioPipeline;
use super::session::DecodeLoopState;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::Packet;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub(super) struct RuntimeAdaptiveProfile {
    pub is_high_res_video: bool,
    pub nominal_fps: f64,
    pub extra_audio_stream_count: usize,
}

pub(super) struct DecodeRuntime {
    pub video_ctx: VideoDecodeContext,
    pub scaler: Option<ScalingContext>,
    pub audio_pipeline: Option<AudioPipeline>,
    pub loop_state: DecodeLoopState,
    /// Holds one muxed packet when draining audio ahead—next loop iteration consumes it before
    /// `read()`, keeping demux order while preventing long video-only stalls on the PCM path.
    pub demux_packet_stash: Option<Packet>,
    pub adaptive_audio_protection_until: Option<Instant>,
    pub adaptive_last_underrun_count: u64,
    pub adaptive_profile: RuntimeAdaptiveProfile,
    pub should_tail_eof: bool,
    pub is_network_source: bool,
    pub is_realtime_source: bool,
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
