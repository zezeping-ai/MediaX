mod cover_art;
mod lyrics;
mod metadata;

pub(crate) use self::cover_art::cover_frame_from_image_bytes;
pub(crate) use self::metadata::load_deferred_audio_cover_frame;
use self::metadata::{build_source_metadata, SourceMetadata};
use super::hw_decode::configure_hw_decode;
use super::output_size::compute_output_size;
use super::types::{HwDecodeStatus, VideoDecodeContext};
use crate::app::media::playback::dto::{HardwareDecodeMode, PlaybackQualityMode};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::codec::threading::{Config as DecoderThreadCfg, Type as DecoderThreadType};
use ffmpeg_next::format;
use ffmpeg_next::format::stream::Disposition;
use ffmpeg_next::media::Type;

pub(crate) fn open_video_decode_context(
    source: &str,
    hw_mode: HardwareDecodeMode,
    quality_mode: PlaybackQualityMode,
    software_fallback_reason: Option<&str>,
    force_audio_only: bool,
) -> Result<VideoDecodeContext, String> {
    ffmpeg::init().map_err(|err| format!("ffmpeg init failed: {err}"))?;
    let input_ctx = format::input(source).map_err(|err| format!("open media failed: {err}"))?;

    let audio_stream_index = input_ctx
        .streams()
        .best(Type::Audio)
        .map(|stream| stream.index());
    let cover_stream_index = input_ctx
        .streams()
        .find(|stream| {
            stream.parameters().medium() == Type::Video
                && stream.disposition().contains(Disposition::ATTACHED_PIC)
        })
        .map(|stream| stream.index());
    let primary_video_stream_index = input_ctx
        .streams()
        .find(|stream| {
            stream.parameters().medium() == Type::Video
                && !stream.disposition().contains(Disposition::ATTACHED_PIC)
        })
        .map(|stream| stream.index());

    if primary_video_stream_index.is_none()
        && audio_stream_index.is_none()
        && cover_stream_index.is_none()
    {
        return Err("no playable audio or video stream found".to_string());
    }

    let source_metadata =
        build_source_metadata(source, &input_ctx, audio_stream_index, cover_stream_index)?;

    if !force_audio_only {
        if let Some(video_stream_index) = primary_video_stream_index {
            return open_primary_video_context(
                input_ctx,
                video_stream_index,
                audio_stream_index,
                quality_mode,
                hw_mode,
                software_fallback_reason,
                source_metadata,
            );
        }
    }

    finalize_audio_only_context(
        input_ctx,
        audio_stream_index,
        quality_mode,
        source_metadata,
        HwDecodeStatus {
            active: false,
            backend: None,
            error: force_audio_only.then(|| {
                software_fallback_reason
                    .unwrap_or("audio-only fallback requested")
                    .to_string()
            }),
            decision: if force_audio_only {
                format!(
                    "forced audio-only fallback: {}",
                    software_fallback_reason.unwrap_or("audio-only fallback requested")
                )
            } else {
                "audio_only_source".to_string()
            },
        },
    )
}

fn open_primary_video_context(
    input_ctx: format::context::Input,
    video_stream_index: usize,
    audio_stream_index: Option<usize>,
    quality_mode: PlaybackQualityMode,
    hw_mode: HardwareDecodeMode,
    software_fallback_reason: Option<&str>,
    source_metadata: SourceMetadata,
) -> Result<VideoDecodeContext, String> {
    let stream_time_base = {
        let input_stream = find_video_stream(&input_ctx, video_stream_index)?;
        input_stream.time_base()
    };
    let fps_value = {
        let input_stream = find_video_stream(&input_ctx, video_stream_index)?;
        stream_fps_value(&input_stream)
    };
    let duration_seconds = {
        let input_stream = find_video_stream(&input_ctx, video_stream_index)?;
        resolve_duration_seconds(&input_ctx, Some(&input_stream))
    };
    let mut codec_context = {
        let input_stream = find_video_stream(&input_ctx, video_stream_index)?;
        codec::context::Context::from_parameters(input_stream.parameters())
            .map_err(|err| format!("decoder context failed: {err}"))?
    };
    let hw_status = configure_hw_decode(&mut codec_context, hw_mode, software_fallback_reason)?;
    // Heavy SW codecs (e.g. 4K HEVC) default to single-threaded decode and fall behind realtime; VT ignores thread flags.
    if !hw_status.active {
        tune_software_decoder_threads(&mut codec_context);
    }
    let decoder = match codec_context.decoder().video() {
        Ok(decoder) => decoder,
        Err(err) if hw_mode == HardwareDecodeMode::Auto && hw_status.active => {
            return open_auto_fallback_video_context(
                input_ctx,
                video_stream_index,
                quality_mode,
                duration_seconds,
                fps_value,
                source_metadata,
                err,
            );
        }
        Err(err) => {
            if audio_stream_index.is_some() {
                return finalize_audio_only_context(
                    input_ctx,
                    audio_stream_index,
                    quality_mode,
                    source_metadata,
                    HwDecodeStatus {
                        active: false,
                        backend: None,
                        error: Some(format!(
                            "video decoder create failed; audio-only fallback: {err}"
                        )),
                        decision: format!(
                            "audio-only fallback after video decoder create failed: {err}"
                        ),
                    },
                );
            }
            return Err(format!("video decoder create failed: {err}"));
        }
    };
    finalize_video_decode_context(
        input_ctx,
        Some(video_stream_index),
        Some(stream_time_base),
        fps_value,
        duration_seconds,
        quality_mode,
        Some(decoder),
        source_metadata,
        hw_status,
    )
}

fn open_auto_fallback_video_context(
    input_ctx: format::context::Input,
    video_stream_index: usize,
    quality_mode: PlaybackQualityMode,
    duration_seconds: f64,
    fps_value: f64,
    source_metadata: SourceMetadata,
    decoder_open_error: ffmpeg::Error,
) -> Result<VideoDecodeContext, String> {
    let fallback_reason =
        format!("auto fallback to software after decoder open failed: {decoder_open_error}");
    let mut software_context = {
        let input_stream = find_video_stream(&input_ctx, video_stream_index)?;
        codec::context::Context::from_parameters(input_stream.parameters())
            .map_err(|ctx_err| format!("decoder context failed: {ctx_err}"))?
    };
    let software_status = configure_hw_decode(
        &mut software_context,
        HardwareDecodeMode::Off,
        Some(&fallback_reason),
    )?;
    tune_software_decoder_threads(&mut software_context);
    let decoder = software_context.decoder().video().map_err(|decode_err| {
        format!("video decoder create failed after fallback: {decode_err}")
    })?;
    let stream_time_base = find_video_stream_time_base(&input_ctx, video_stream_index)?;
    finalize_video_decode_context(
        input_ctx,
        Some(video_stream_index),
        Some(stream_time_base),
        fps_value,
        duration_seconds,
        quality_mode,
        Some(decoder),
        source_metadata,
        software_status,
    )
}

fn finalize_audio_only_context(
    input_ctx: format::context::Input,
    audio_stream_index: Option<usize>,
    quality_mode: PlaybackQualityMode,
    source_metadata: SourceMetadata,
    hw_status: HwDecodeStatus,
) -> Result<VideoDecodeContext, String> {
    let audio_duration_stream = audio_stream_index
        .and_then(|index| input_ctx.streams().find(|stream| stream.index() == index));
    let duration_seconds = resolve_duration_seconds(&input_ctx, audio_duration_stream.as_ref());
    finalize_video_decode_context(
        input_ctx,
        None,
        None,
        0.0,
        duration_seconds,
        quality_mode,
        None,
        source_metadata,
        hw_status,
    )
}

fn finalize_video_decode_context(
    input_ctx: format::context::Input,
    video_stream_index: Option<usize>,
    video_time_base: Option<ffmpeg::Rational>,
    fps_value: f64,
    duration_seconds: f64,
    quality_mode: PlaybackQualityMode,
    decoder: Option<ffmpeg::decoder::Video>,
    source_metadata: SourceMetadata,
    hw_status: HwDecodeStatus,
) -> Result<VideoDecodeContext, String> {
    let (output_width, output_height) = decoder
        .as_ref()
        .map(|value| compute_output_size(value.width(), value.height(), quality_mode))
        .or_else(|| {
            source_metadata
                .cover_frame
                .as_ref()
                .map(|frame| (frame.width, frame.height))
        })
        .unwrap_or((0, 0));
    Ok(VideoDecodeContext {
        input_ctx,
        video_stream_index,
        video_time_base,
        decoder,
        fps_value,
        duration_seconds,
        output_width,
        output_height,
        media_kind: source_metadata.media_kind,
        has_cover_art: source_metadata.has_cover_art,
        cover_frame: source_metadata.cover_frame,
        deferred_cover_bytes: source_metadata.deferred_cover_bytes,
        title: source_metadata.title,
        artist: source_metadata.artist,
        album: source_metadata.album,
        lyrics: source_metadata.lyrics,
        hw_decode_active: hw_status.active,
        hw_decode_backend: hw_status.backend,
        hw_decode_error: hw_status.error,
        hw_decode_decision: hw_status.decision,
    })
}

fn tune_software_decoder_threads(ctx: &mut codec::context::Context) {
    let count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 16);
    ctx.set_threading(DecoderThreadCfg {
        kind: DecoderThreadType::Frame,
        count,
        ..DecoderThreadCfg::default()
    });
}

fn find_video_stream<'a>(
    input_ctx: &'a format::context::Input,
    video_stream_index: usize,
) -> Result<ffmpeg::Stream<'a>, String> {
    input_ctx
        .streams()
        .find(|stream| stream.index() == video_stream_index)
        .ok_or_else(|| format!("video stream {video_stream_index} not found"))
}

fn find_video_stream_time_base(
    input_ctx: &format::context::Input,
    video_stream_index: usize,
) -> Result<ffmpeg::Rational, String> {
    Ok(find_video_stream(input_ctx, video_stream_index)?.time_base())
}

fn resolve_duration_seconds(
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

fn stream_fps_value(stream: &ffmpeg::Stream<'_>) -> f64 {
    let fps = stream.avg_frame_rate();
    if fps.denominator() != 0 {
        f64::from(fps.numerator()) / f64::from(fps.denominator())
    } else {
        0.0
    }
}
