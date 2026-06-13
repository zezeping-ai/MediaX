use ffmpeg_next::decoder::Audio;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::format::context::Input;

/// Unified playback timeline: both streams map raw decoder PTS to the same master axis.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaybackTimeline {
    /// Subtracted from raw audio PTS (codec priming + stream-start skew).
    pub audio_pts_offset_seconds: f64,
    /// Subtracted from raw video PTS. Negative when video starts after audio in the container.
    pub video_pts_offset_seconds: f64,
    pub video_starts_after_audio_seconds: f64,
    pub audio_starts_after_video_seconds: f64,
}

impl PlaybackTimeline {
    pub fn resolve(
        input_ctx: &Input,
        audio_stream_index: usize,
        video_stream_index: Option<usize>,
        decoder: &Audio,
    ) -> Self {
        let (video_starts_after_audio_seconds, audio_starts_after_video_seconds) =
            stream_start_offsets_seconds(input_ctx, audio_stream_index, video_stream_index);
        let codec_priming_seconds = decoder_priming_seconds(decoder);
        Self {
            audio_pts_offset_seconds: (codec_priming_seconds + audio_starts_after_video_seconds)
                .max(0.0),
            // Negative offset shifts video playback time forward when the stream starts later.
            video_pts_offset_seconds: -video_starts_after_audio_seconds,
            video_starts_after_audio_seconds,
            audio_starts_after_video_seconds,
        }
    }

    pub fn normalize_audio_pts(&self, raw_seconds: f64) -> f64 {
        normalize_pts(raw_seconds, self.audio_pts_offset_seconds)
    }

    pub fn normalize_video_pts(&self, raw_seconds: f64) -> f64 {
        normalize_pts(raw_seconds, self.video_pts_offset_seconds)
    }

    pub fn describe(&self) -> String {
        format!(
            "audio_pts_offset_ms={:.1} video_pts_offset_ms={:.1} video_late_ms={:.1} audio_late_ms={:.1}",
            self.audio_pts_offset_seconds * 1000.0,
            self.video_pts_offset_seconds * 1000.0,
            self.video_starts_after_audio_seconds * 1000.0,
            self.audio_starts_after_video_seconds * 1000.0,
        )
    }
}

fn normalize_pts(raw_seconds: f64, pts_offset_seconds: f64) -> f64 {
    if !raw_seconds.is_finite() {
        return 0.0;
    }
    (raw_seconds - pts_offset_seconds).max(0.0)
}

fn decoder_priming_seconds(decoder: &Audio) -> f64 {
    let sample_rate = decoder.rate();
    if sample_rate == 0 {
        return 0.0;
    }
    let priming_samples = unsafe { decoder_priming_samples(decoder) };
    if priming_samples <= 0 {
        return 0.0;
    }
    priming_samples as f64 / sample_rate as f64
}

unsafe fn decoder_priming_samples(decoder: &Audio) -> i64 {
    let ptr = decoder.as_ptr();
    if ptr.is_null() {
        return 0;
    }
    let ctx = &*ptr;
    i64::from(ctx.delay.max(0)).saturating_add(i64::from(ctx.initial_padding.max(0)))
}

fn stream_start_offsets_seconds(
    input_ctx: &Input,
    audio_stream_index: usize,
    video_stream_index: Option<usize>,
) -> (f64, f64) {
    let Some(video_stream_index) = video_stream_index else {
        return (0.0, 0.0);
    };
    let audio_start = input_ctx
        .streams()
        .find(|stream| stream.index() == audio_stream_index)
        .map(|stream| stream_start_seconds(&stream))
        .unwrap_or(0.0);
    let video_start = input_ctx
        .streams()
        .find(|stream| stream.index() == video_stream_index)
        .map(|stream| stream_start_seconds(&stream))
        .unwrap_or(0.0);
    (
        (video_start - audio_start).max(0.0),
        (audio_start - video_start).max(0.0),
    )
}

fn stream_start_seconds(stream: &format::stream::Stream) -> f64 {
    let start = stream.start_time();
    if start == ffi::AV_NOPTS_VALUE {
        return 0.0;
    }
    start as f64 * f64::from(stream.time_base())
}
