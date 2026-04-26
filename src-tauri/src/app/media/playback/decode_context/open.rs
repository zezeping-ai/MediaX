use super::hw_decode::configure_hw_decode;
use super::output_size::compute_output_size;
use super::types::{HwDecodeStatus, VideoDecodeContext};
use crate::app::media::model::{HardwareDecodeMode, MediaLyricLine, PlaybackMediaKind, PlaybackQualityMode};
use crate::app::media::playback::render::renderer::VideoFrame;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::format::stream::Disposition;
use ffmpeg_next::media::Type;
use image::imageops::FilterType;
use std::path::{Path, PathBuf};

const COVER_MAX_EDGE: u32 = 1440;
const LYRIC_METADATA_KEYS: &[&str] = &[
    "lyrics",
    "syncedlyrics",
    "lrc",
    "LYRICS",
    "SYNCEDLYRICS",
];
const TITLE_METADATA_KEYS: &[&str] = &["title", "TITLE"];
const ARTIST_METADATA_KEYS: &[&str] = &["artist", "ARTIST", "album_artist", "ALBUMARTIST"];
const ALBUM_METADATA_KEYS: &[&str] = &["album", "ALBUM"];

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
    let primary_video_stream = input_ctx.streams().find(|stream| {
        stream.parameters().medium() == Type::Video
            && !stream.disposition().contains(Disposition::ATTACHED_PIC)
    });

    if primary_video_stream.is_none() && audio_stream_index.is_none() && cover_stream_index.is_none() {
        return Err("no playable audio or video stream found".to_string());
    }

    let source_metadata = build_source_metadata(source, &input_ctx, audio_stream_index, cover_stream_index)?;

    if !force_audio_only {
        if let Some(input_stream) = primary_video_stream {
        let video_stream_index = input_stream.index();
        let stream_time_base = input_stream.time_base();
        let fps = input_stream.avg_frame_rate();
        let fps_value = if fps.denominator() != 0 {
            f64::from(fps.numerator()) / f64::from(fps.denominator())
        } else {
            0.0
        };
        let duration_seconds = resolve_duration_seconds(&input_ctx, Some(&input_stream));
        let mut codec_context = codec::context::Context::from_parameters(input_stream.parameters())
            .map_err(|err| format!("decoder context failed: {err}"))?;
        let hw_status = configure_hw_decode(&mut codec_context, hw_mode, software_fallback_reason)?;
        let decoder = match codec_context.decoder().video() {
            Ok(decoder) => decoder,
            Err(err) if hw_mode == HardwareDecodeMode::Auto && hw_status.active => {
                let fallback_reason =
                    format!("auto fallback to software after decoder open failed: {err}");
                let mut software_context =
                    codec::context::Context::from_parameters(input_stream.parameters())
                        .map_err(|ctx_err| format!("decoder context failed: {ctx_err}"))?;
                let software_status = configure_hw_decode(
                    &mut software_context,
                    HardwareDecodeMode::Off,
                    Some(&fallback_reason),
                )?;
                let decoder = software_context
                    .decoder()
                    .video()
                    .map_err(|decode_err| {
                        format!("video decoder create failed after fallback: {decode_err}")
                    })?;
                return finalize_video_decode_context(
                    input_ctx,
                    Some(video_stream_index),
                    Some(stream_time_base),
                    fps_value,
                    duration_seconds,
                    quality_mode,
                    Some(decoder),
                    source_metadata,
                    software_status,
                );
            }
            Err(err) => {
                if audio_stream_index.is_some() {
                    let audio_duration_stream = audio_stream_index
                        .and_then(|index| input_ctx.streams().find(|stream| stream.index() == index));
                    let duration_seconds =
                        resolve_duration_seconds(&input_ctx, audio_duration_stream.as_ref());
                    return finalize_video_decode_context(
                        input_ctx,
                        None,
                        None,
                        0.0,
                        duration_seconds,
                        quality_mode,
                        None,
                        source_metadata,
                        HwDecodeStatus {
                            active: false,
                            backend: None,
                            error: Some(format!("video decoder create failed; audio-only fallback: {err}")),
                            decision: format!("audio-only fallback after video decoder create failed: {err}"),
                        },
                    );
                }
                return Err(format!("video decoder create failed: {err}"));
            }
        };
            return finalize_video_decode_context(
                input_ctx,
                Some(video_stream_index),
                Some(stream_time_base),
                fps_value,
                duration_seconds,
                quality_mode,
                Some(decoder),
                source_metadata,
                hw_status,
            );
        }
    }

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
        HwDecodeStatus {
            active: false,
            backend: None,
            error: force_audio_only
                .then(|| software_fallback_reason.unwrap_or("audio-only fallback requested").to_string()),
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

struct SourceMetadata {
    media_kind: PlaybackMediaKind,
    has_cover_art: bool,
    cover_frame: Option<VideoFrame>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    lyrics: Vec<MediaLyricLine>,
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
        .or_else(|| source_metadata.cover_frame.as_ref().map(|frame| (frame.width, frame.height)))
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

fn resolve_duration_seconds(
    input_ctx: &format::context::Input,
    stream: Option<&ffmpeg_next::format::stream::Stream<'_>>,
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

fn build_source_metadata(
    source: &str,
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    cover_stream_index: Option<usize>,
) -> Result<SourceMetadata, String> {
    let container_metadata = input_ctx.metadata();
    let title = first_metadata_value(&container_metadata, TITLE_METADATA_KEYS).or_else(|| {
        audio_stream_index
            .and_then(|index| input_ctx.streams().find(|stream| stream.index() == index))
            .and_then(|stream| first_metadata_value(&stream.metadata(), TITLE_METADATA_KEYS))
    });
    let artist = first_metadata_value(&container_metadata, ARTIST_METADATA_KEYS).or_else(|| {
        audio_stream_index
            .and_then(|index| input_ctx.streams().find(|stream| stream.index() == index))
            .and_then(|stream| first_metadata_value(&stream.metadata(), ARTIST_METADATA_KEYS))
    });
    let album = first_metadata_value(&container_metadata, ALBUM_METADATA_KEYS).or_else(|| {
        audio_stream_index
            .and_then(|index| input_ctx.streams().find(|stream| stream.index() == index))
            .and_then(|stream| first_metadata_value(&stream.metadata(), ALBUM_METADATA_KEYS))
    });

    let mut lyrics = load_sidecar_lrc(source)?;
    if lyrics.is_empty() {
        if let Some(stream) =
            audio_stream_index.and_then(|index| input_ctx.streams().find(|value| value.index() == index))
        {
            lyrics = extract_lyrics_from_metadata(&stream.metadata());
        }
    }
    if lyrics.is_empty() {
        lyrics = extract_lyrics_from_metadata(&container_metadata);
    }

    let cover_frame = cover_stream_index
        .and_then(|index| extract_cover_packet_bytes(input_ctx, index))
        .and_then(|bytes| cover_frame_from_image_bytes(&bytes).ok());

    Ok(SourceMetadata {
        media_kind: if audio_stream_index.is_some() && input_ctx.streams().find(|stream| {
            stream.parameters().medium() == Type::Video
                && !stream.disposition().contains(Disposition::ATTACHED_PIC)
        }).is_none() {
            PlaybackMediaKind::Audio
        } else {
            PlaybackMediaKind::Video
        },
        has_cover_art: cover_frame.is_some(),
        cover_frame,
        title,
        artist,
        album,
        lyrics,
    })
}

fn first_metadata_value(
    dictionary: &ffmpeg::DictionaryRef<'_>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| dictionary.get(key))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn load_sidecar_lrc(source: &str) -> Result<Vec<MediaLyricLine>, String> {
    let path = Path::new(source);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(Vec::new());
    };
    let mut lrc_path = PathBuf::from(path);
    lrc_path.set_file_name(format!("{stem}.lrc"));
    if !lrc_path.exists() {
        return Ok(Vec::new());
    }
    let contents = std::fs::read_to_string(&lrc_path)
        .map_err(|err| format!("read lyric file failed: {err}"))?;
    Ok(parse_lrc_contents(&contents))
}

fn extract_lyrics_from_metadata(dictionary: &ffmpeg::DictionaryRef<'_>) -> Vec<MediaLyricLine> {
    for key in LYRIC_METADATA_KEYS {
        if let Some(value) = dictionary.get(key) {
            let parsed = parse_lrc_contents(value);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn parse_lrc_contents(contents: &str) -> Vec<MediaLyricLine> {
    let mut lines = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let mut rest = line;
        let mut timestamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some((ts, tail)) = stripped.split_once(']') else {
                break;
            };
            let Some(seconds) = parse_lrc_timestamp(ts) else {
                break;
            };
            timestamps.push(seconds);
            rest = tail.trim_start();
        }
        if timestamps.is_empty() || rest.is_empty() {
            continue;
        }
        for timestamp in timestamps {
            lines.push(MediaLyricLine {
                time_seconds: timestamp,
                text: rest.to_string(),
            });
        }
    }
    lines.sort_by(|a, b| {
        a.time_seconds
            .partial_cmp(&b.time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    lines
}

fn parse_lrc_timestamp(value: &str) -> Option<f64> {
    let parts: Vec<&str> = value.split(':').collect();
    match parts.as_slice() {
        [minutes, seconds] => {
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(minutes * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<f64>().ok()?;
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

fn extract_cover_packet_bytes(
    input_ctx: &format::context::Input,
    stream_index: usize,
) -> Option<Vec<u8>> {
    let stream = input_ctx.streams().find(|value| value.index() == stream_index)?;
    // SAFETY: `stream` comes from the live format context and `attached_pic` is read-only.
    let packet = unsafe { &(*stream.as_ptr()).attached_pic };
    if packet.data.is_null() || packet.size <= 0 {
        return None;
    }
    // SAFETY: FFmpeg owns the packet buffer for the lifetime of the input context.
    let bytes = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) };
    Some(bytes.to_vec())
}

fn cover_frame_from_image_bytes(bytes: &[u8]) -> Result<VideoFrame, String> {
    let image = image::load_from_memory(bytes)
        .map_err(|err| format!("decode cover art failed: {err}"))?;
    let resized = image.resize(COVER_MAX_EDGE, COVER_MAX_EDGE, FilterType::Lanczos3);
    let rgb = resized.to_rgb8();
    let mut width = rgb.width().max(2);
    let mut height = rgb.height().max(2);
    if width % 2 != 0 {
        width += 1;
    }
    if height % 2 != 0 {
        height += 1;
    }
    let resized = if width != rgb.width() || height != rgb.height() {
        image::imageops::resize(&rgb, width, height, FilterType::Lanczos3)
    } else {
        rgb
    };
    let width_usize = width as usize;
    let height_usize = height as usize;
    let mut y_plane = Vec::with_capacity(width_usize * height_usize);
    let mut uv_plane = Vec::with_capacity(width_usize * height_usize / 2);

    for y in 0..height {
        for x in 0..width {
            let pixel = resized.get_pixel(x, y);
            let (r, g, b) = (pixel[0] as f32, pixel[1] as f32, pixel[2] as f32);
            let luma = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
            y_plane.push(luma);
        }
    }

    for y in (0..height).step_by(2) {
        for x in (0..width).step_by(2) {
            let mut r_sum = 0.0;
            let mut g_sum = 0.0;
            let mut b_sum = 0.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let pixel = resized.get_pixel(x + dx, y + dy);
                    r_sum += pixel[0] as f32;
                    g_sum += pixel[1] as f32;
                    b_sum += pixel[2] as f32;
                }
            }
            let r = r_sum / 4.0;
            let g = g_sum / 4.0;
            let b = b_sum / 4.0;
            let u = (-0.168736 * r - 0.331264 * g + 0.5 * b + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            let v = (0.5 * r - 0.418688 * g - 0.081312 * b + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            uv_plane.push(u);
            uv_plane.push(v);
        }
    }

    Ok(VideoFrame {
        pts_seconds: 0.0,
        width,
        height,
        y_plane,
        uv_plane,
        color_matrix: [
            [1.0, 0.0, 1.402],
            [1.0, -0.344136, -0.714136],
            [1.0, 1.772, 0.0],
        ],
        y_offset: 0.0,
        y_scale: 1.0,
        uv_offset: 0.5,
        uv_scale: 1.0,
    })
}
