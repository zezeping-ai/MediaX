use crate::app::media::model::MediaLyricLine;
use ffmpeg_next as ffmpeg;
use std::path::{Path, PathBuf};

const LYRIC_METADATA_KEYS: &[&str] = &[
    "lyrics",
    "syncedlyrics",
    "lrc",
    "LYRICS",
    "SYNCEDLYRICS",
];

pub(super) fn load_source_lyrics(
    source: &str,
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream_index: Option<usize>,
) -> Result<Vec<MediaLyricLine>, String> {
    let mut lyrics = load_sidecar_lrc(source)?;
    if lyrics.is_empty() {
        if let Some(stream) =
            audio_stream_index.and_then(|index| input_ctx.streams().find(|value| value.index() == index))
        {
            lyrics = extract_lyrics_from_metadata(&stream.metadata());
        }
    }
    if lyrics.is_empty() {
        lyrics = extract_lyrics_from_metadata(&input_ctx.metadata());
    }
    Ok(lyrics)
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
