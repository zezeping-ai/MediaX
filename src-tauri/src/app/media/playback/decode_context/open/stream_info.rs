use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;

pub(super) fn find_video_stream<'a>(
    input_ctx: &'a format::context::Input,
    video_stream_index: usize,
) -> Result<ffmpeg::Stream<'a>, String> {
    input_ctx
        .streams()
        .find(|stream| stream.index() == video_stream_index)
        .ok_or_else(|| format!("video stream {video_stream_index} not found"))
}

pub(super) fn find_video_stream_time_base(
    input_ctx: &format::context::Input,
    video_stream_index: usize,
) -> Result<ffmpeg::Rational, String> {
    Ok(find_video_stream(input_ctx, video_stream_index)?.time_base())
}

pub(super) fn resolve_duration_seconds(
    input_ctx: &format::context::Input,
    stream: Option<&ffmpeg::format::stream::Stream<'_>>,
) -> f64 {
    if let Some(stream) = stream {
        let stream_duration = stream.duration();
        if stream_duration > 0 {
            return (stream_duration as f64) * f64::from(stream.time_base());
        }
    }
    let container_duration = input_ctx.duration();
    if container_duration > 0 {
        return (container_duration as f64) / f64::from(ffmpeg::ffi::AV_TIME_BASE);
    }
    0.0
}

pub(super) fn stream_fps_value(stream: &ffmpeg::Stream<'_>) -> f64 {
    let fps = stream.avg_frame_rate();
    if fps.denominator() != 0 {
        f64::from(fps.numerator()) / f64::from(fps.denominator())
    } else {
        0.0
    }
}
