use crate::app::media::model::MediaLyricLine;
use ffmpeg_next as ffmpeg;
use std::path::{Path, PathBuf};

use crate::app::media::lyrics::parse_lrc_contents;

use super::metadata::dictionary_value_lossy;

const LYRIC_METADATA_KEYS: &[&str] = &["lyrics", "syncedlyrics", "lrc", "LYRICS", "SYNCEDLYRICS"];

pub(super) struct LocalLyricsLoadResult {
    pub lines: Vec<MediaLyricLine>,
    pub source: Option<String>,
}

pub(super) fn load_source_lyrics(
    source: &str,
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream_index: Option<usize>,
) -> Result<LocalLyricsLoadResult, String> {
    let embedded = load_embedded_lyrics(input_ctx, audio_stream_index);
    Ok(pick_preferred_local_lyrics(embedded, load_sidecar_lrc(source)?))
}

fn load_embedded_lyrics(
    input_ctx: &ffmpeg::format::context::Input,
    audio_stream_index: Option<usize>,
) -> LocalLyricsLoadResult {
    if let Some(stream) = audio_stream_index
        .and_then(|index| input_ctx.streams().find(|value| value.index() == index))
    {
        let lyrics = extract_lyrics_from_metadata(&stream.metadata());
        if !lyrics.lines.is_empty() {
            return lyrics;
        }
    }
    extract_lyrics_from_metadata(&input_ctx.metadata())
}

fn load_sidecar_lrc(source: &str) -> Result<LocalLyricsLoadResult, String> {
    let path = Path::new(source);
    if !path.is_file() {
        return Ok(LocalLyricsLoadResult {
            lines: Vec::new(),
            source: None,
        });
    }
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(LocalLyricsLoadResult {
            lines: Vec::new(),
            source: None,
        });
    };
    let mut lrc_path = PathBuf::from(path);
    lrc_path.set_file_name(format!("{stem}.lrc"));
    if !lrc_path.exists() {
        return Ok(LocalLyricsLoadResult {
            lines: Vec::new(),
            source: None,
        });
    }
    let contents = std::fs::read_to_string(&lrc_path)
        .map_err(|err| format!("read lyric file failed: {err}"))?;
    Ok(LocalLyricsLoadResult {
        lines: parse_lrc_contents(&contents),
        source: Some("sidecar".to_string()),
    })
}

fn extract_lyrics_from_metadata(dictionary: &ffmpeg::DictionaryRef<'_>) -> LocalLyricsLoadResult {
    for key in LYRIC_METADATA_KEYS {
        if let Some(value) = dictionary_value_lossy(dictionary, key) {
            let parsed = parse_lrc_contents(&value);
            if !parsed.is_empty() {
                return LocalLyricsLoadResult {
                    lines: parsed,
                    source: Some("embedded".to_string()),
                };
            }
        }
    }
    LocalLyricsLoadResult {
        lines: Vec::new(),
        source: None,
    }
}

fn pick_preferred_local_lyrics(
    embedded: LocalLyricsLoadResult,
    sidecar: LocalLyricsLoadResult,
) -> LocalLyricsLoadResult {
    if !embedded.lines.is_empty() {
        embedded
    } else {
        sidecar
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalLyricsLoadResult, pick_preferred_local_lyrics};

    fn result(source: &str, text: &str) -> LocalLyricsLoadResult {
        LocalLyricsLoadResult {
            lines: vec![crate::app::media::model::MediaLyricLine {
                time_seconds: 0.0,
                text: text.to_string(),
            }],
            source: Some(source.to_string()),
        }
    }

    fn empty() -> LocalLyricsLoadResult {
        LocalLyricsLoadResult {
            lines: Vec::new(),
            source: None,
        }
    }

    #[test]
    fn prefers_embedded_over_sidecar() {
        let embedded = result("embedded", "内嵌");
        let sidecar = result("sidecar", "同目录");
        let picked = pick_preferred_local_lyrics(embedded, sidecar);
        assert_eq!(picked.source.as_deref(), Some("embedded"));
        assert_eq!(picked.lines[0].text, "内嵌");
    }

    #[test]
    fn falls_back_to_sidecar_when_embedded_missing() {
        let picked = pick_preferred_local_lyrics(empty(), result("sidecar", "同目录"));
        assert_eq!(picked.source.as_deref(), Some("sidecar"));
    }
}
