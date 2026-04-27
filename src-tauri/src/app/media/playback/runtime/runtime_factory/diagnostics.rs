use super::emit_debug;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use ffmpeg_next::codec;
use ffmpeg_next::Stream;
use tauri::AppHandle;

pub(super) fn emit_video_stream_diagnostics(app: &AppHandle, video_ctx: &VideoDecodeContext) {
    let Some(decoder) = video_ctx.decoder.as_ref() else {
        emit_debug(app, "video", "no video stream");
        return;
    };
    emit_video_codec_profile(app, decoder);

    let Some(video_stream) = resolve_video_stream(video_ctx) else {
        return;
    };
    emit_video_stream_format(app, video_ctx, decoder, &video_stream);
    emit_video_stream_timing(app, &video_stream);
}

pub(super) fn emit_audio_stream_diagnostics(
    app: &AppHandle,
    video_ctx: &VideoDecodeContext,
    audio_stream_index: Option<usize>,
) {
    emit_debug(
        app,
        "audio",
        match audio_stream_index {
            Some(index) => format!("audio stream index={index}"),
            None => "no audio stream".to_string(),
        },
    );
    let Some(audio_index) = audio_stream_index else {
        return;
    };
    let Some(audio_stream) = video_ctx
        .input_ctx
        .streams()
        .find(|stream| stream.index() == audio_index)
    else {
        return;
    };

    let audio_tb = audio_stream.time_base();
    let audio_codec = audio_stream.parameters().id();
    let audio_details = codec::context::Context::from_parameters(audio_stream.parameters())
        .ok()
        .and_then(|ctx| ctx.decoder().audio().ok());
    if let Some(audio_decoder) = audio_details {
        let channels = audio_decoder.channels();
        let sample_rate = audio_decoder.rate();
        let sample_fmt = audio_decoder.format();
        let channel_layout = if audio_decoder.channel_layout().is_empty() {
            format!("{}ch", channels)
        } else {
            format!("{:?}", audio_decoder.channel_layout())
        };
        emit_debug(
            app,
            "audio_format",
            format!(
                "codec={:?} sample_rate={}Hz channels={} layout={} sample_fmt={:?} tb={}/{}",
                audio_codec,
                sample_rate,
                channels,
                channel_layout,
                sample_fmt,
                audio_tb.numerator(),
                audio_tb.denominator()
            ),
        );
        return;
    }
    emit_debug(
        app,
        "audio_format",
        format!(
            "codec={:?} tb={}/{}",
            audio_codec,
            audio_tb.numerator(),
            audio_tb.denominator()
        ),
    );
}

fn emit_video_codec_profile(app: &AppHandle, decoder: &ffmpeg_next::decoder::Video) {
    // SAFETY: decoder pointer is valid while `video_ctx.decoder` is alive; read-only access.
    let (profile, level, has_b_frames) = unsafe {
        let raw = &*decoder.as_ptr();
        (raw.profile, raw.level, raw.has_b_frames)
    };
    emit_debug(
        app,
        "video_codec_profile",
        format!(
            "codec={:?} profile={} level={} has_b_frames={}",
            decoder.id(),
            profile,
            level,
            has_b_frames
        ),
    );
}

fn resolve_video_stream(video_ctx: &VideoDecodeContext) -> Option<Stream<'_>> {
    let video_stream_index = video_ctx.video_stream_index?;
    video_ctx
        .input_ctx
        .streams()
        .find(|stream| stream.index() == video_stream_index)
}

fn emit_video_stream_format(
    app: &AppHandle,
    video_ctx: &VideoDecodeContext,
    decoder: &ffmpeg_next::decoder::Video,
    video_stream: &Stream<'_>,
) {
    emit_debug(
        app,
        "video_format",
        format!(
            "container={} codec={:?} pixel_fmt={:?}",
            video_ctx.input_ctx.format().name(),
            video_stream.parameters().id(),
            decoder.format()
        ),
    );
}

fn emit_video_stream_timing(app: &AppHandle, video_stream: &Stream<'_>) {
    let time_base = video_stream.time_base();
    let avg_frame_rate = video_stream.avg_frame_rate();
    let nominal_frame_rate = video_stream.rate();
    let duration_ts = video_stream.duration();
    let duration_seconds = if duration_ts > 0 {
        (duration_ts as f64) * f64::from(time_base)
    } else {
        0.0
    };
    emit_debug(
        app,
        "video_stream",
        format!(
            "codec={:?} tb={}/{} avg_fps={:.3} nominal_fps={:.3} duration={:.3}s duration_ts={} start_ts={}",
            video_stream.parameters().id(),
            time_base.numerator(),
            time_base.denominator(),
            rational_to_f64(avg_frame_rate),
            rational_to_f64(nominal_frame_rate),
            duration_seconds,
            duration_ts,
            video_stream.start_time()
        ),
    );
}

fn rational_to_f64(value: ffmpeg_next::Rational) -> f64 {
    if value.denominator() == 0 {
        return 0.0;
    }
    f64::from(value.numerator()) / f64::from(value.denominator())
}
