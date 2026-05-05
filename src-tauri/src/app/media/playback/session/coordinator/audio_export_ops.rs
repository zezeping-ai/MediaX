use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::media::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

pub fn export_current_audio_track(
    state: State<'_, MediaState>,
    output_dir: String,
) -> MediaResult<String> {
    let (source, media_kind) = {
        let playback = state::playback(&state)?;
        let model = playback.state();
        let current = model
            .current_path
            .clone()
            .ok_or_else(|| MediaError::invalid_input("no active source to export"))?;
        (current, model.media_kind)
    };
    if media_kind != PlaybackMediaKind::Video {
        return Err(MediaError::invalid_input(
            "audio export is only available for video sources",
        ));
    }
    let output_dir = output_dir.trim();
    if output_dir.is_empty() {
        return Err(MediaError::invalid_input("audio export output_dir is required"));
    }
    fs::create_dir_all(output_dir).map_err(|err| {
        MediaError::internal(format!(
            "failed to create audio export directory '{}': {err}",
            output_dir
        ))
    })?;
    extract_audio_stream_to_directory(&source, output_dir).map_err(MediaError::internal)
}

fn extract_audio_stream_to_directory(source: &str, output_dir: &str) -> Result<String, String> {
    let mut input_ctx =
        format::input(source).map_err(|err| format!("open input source failed: {err}"))?;
    let audio_stream = input_ctx
        .streams()
        .best(Type::Audio)
        .ok_or_else(|| "no audio stream found in current source".to_string())?;
    let audio_stream_index = audio_stream.index();
    let audio_time_base = audio_stream.time_base();
    let audio_codec_id = audio_stream.parameters().id();
    let output_ext = audio_extension_for_codec(audio_codec_id);
    let output_path = next_available_export_path(output_dir, source, output_ext)?;

    let mut output_ctx = format::output(&output_path)
        .map_err(|err| format!("open export output failed: {err}"))?;
    let out_stream_index = {
        let mut out_stream = output_ctx
            .add_stream(codec::encoder::find(codec::Id::None))
            .map_err(|err| format!("add export stream failed: {err}"))?;
        out_stream.set_parameters(audio_stream.parameters());
        out_stream.set_time_base(audio_time_base);
        out_stream.index()
    };
    let out_time_base = output_ctx
        .stream(out_stream_index)
        .ok_or_else(|| "export output stream not found".to_string())?
        .time_base();

    output_ctx
        .write_header()
        .map_err(|err| format!("write export header failed: {err}"))?;

    for (stream, mut packet) in input_ctx.packets() {
        if stream.index() != audio_stream_index {
            continue;
        }
        packet.set_stream(out_stream_index);
        packet.rescale_ts(audio_time_base, out_time_base);
        packet.set_position(-1);
        packet
            .write_interleaved(&mut output_ctx)
            .map_err(|err| format!("write export packet failed: {err}"))?;
    }

    output_ctx
        .write_trailer()
        .map_err(|err| format!("write export trailer failed: {err}"))?;
    Ok(output_path.display().to_string())
}

fn next_available_export_path(
    output_dir: &str,
    source: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    let base_name = source_basename(source);
    let mut candidate = Path::new(output_dir).join(format!("{base_name}.{extension}"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    candidate = Path::new(output_dir).join(format!("{base_name}-{timestamp}.{extension}"));
    Ok(candidate)
}

fn source_basename(source: &str) -> String {
    let no_query = source
        .split(['?', '#'])
        .next()
        .unwrap_or(source)
        .trim_end_matches('/');
    let raw_name = no_query
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("mediax-audio");
    let stem = Path::new(raw_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("mediax-audio");
    let sanitized = stem
        .chars()
        .map(|ch| if r#"\/:*?"<>|"#.contains(ch) { '-' } else { ch })
        .collect::<String>()
        .trim()
        .to_string();
    if sanitized.is_empty() {
        "mediax-audio".to_string()
    } else {
        format!("{sanitized}-audio")
    }
}

fn audio_extension_for_codec(codec_id: codec::Id) -> &'static str {
    match codec_id {
        codec::Id::MP3 => "mp3",
        codec::Id::AAC => "m4a",
        codec::Id::FLAC => "flac",
        codec::Id::OPUS => "opus",
        codec::Id::VORBIS => "ogg",
        codec::Id::PCM_S16LE
        | codec::Id::PCM_S24LE
        | codec::Id::PCM_S32LE
        | codec::Id::PCM_F32LE
        | codec::Id::PCM_F64LE => "wav",
        _ => "mka",
    }
}
