use crate::app::media::playback::sync::PlaybackTimeline;
use ffmpeg_next::decoder::Audio;
use ffmpeg_next::format::context::Input;

pub(crate) fn resolve_playback_timeline(
    input_ctx: &Input,
    audio_stream_index: usize,
    video_stream_index: Option<usize>,
    decoder: &Audio,
) -> PlaybackTimeline {
    PlaybackTimeline::resolve(
        input_ctx,
        audio_stream_index,
        video_stream_index,
        decoder,
    )
}

pub(crate) fn adjust_audio_pts_seconds(raw_seconds: f64, pts_offset_seconds: f64) -> f64 {
    PlaybackTimeline {
        audio_pts_offset_seconds: pts_offset_seconds.max(0.0),
        video_pts_offset_seconds: 0.0,
        video_starts_after_audio_seconds: 0.0,
        audio_starts_after_video_seconds: 0.0,
    }
    .normalize_audio_pts(raw_seconds)
}
