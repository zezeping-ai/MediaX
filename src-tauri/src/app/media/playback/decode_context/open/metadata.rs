use super::cover_art::extract_cover_frame;
use super::lyrics::load_source_lyrics;
use crate::app::media::model::MediaLyricLine;
use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::render::renderer::VideoFrame;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::format::stream::Disposition;
use ffmpeg_next::media::Type;
use std::ffi::{CStr, CString};
use std::ptr;

const TITLE_METADATA_KEYS: &[&str] = &["title", "TITLE"];
const ARTIST_METADATA_KEYS: &[&str] = &["artist", "ARTIST", "album_artist", "ALBUMARTIST"];
const ALBUM_METADATA_KEYS: &[&str] = &["album", "ALBUM"];

pub(crate) struct SourceMetadata {
    pub(crate) media_kind: PlaybackMediaKind,
    pub(crate) has_cover_art: bool,
    pub(crate) cover_frame: Option<VideoFrame>,
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) lyrics: Vec<MediaLyricLine>,
    pub(crate) lyrics_source: Option<String>,
}

pub(super) fn build_source_metadata(
    source: &str,
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    cover_stream_index: Option<usize>,
) -> Result<SourceMetadata, String> {
    let media_kind = resolve_media_kind(input_ctx, audio_stream_index);
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
    let lyrics_result = load_source_lyrics(source, input_ctx, audio_stream_index)?;
    let cover_frame = extract_cover_frame(input_ctx, cover_stream_index);
    let has_cover_art = cover_frame.is_some();

    Ok(SourceMetadata {
        media_kind,
        has_cover_art,
        cover_frame,
        title,
        artist,
        album,
        lyrics: lyrics_result.lines,
        lyrics_source: lyrics_result.source,
    })
}

fn first_metadata_value(dictionary: &ffmpeg::DictionaryRef<'_>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| dictionary_value_lossy(dictionary, key))
        .and_then(|value| normalize_metadata_value(&value))
}

pub(super) fn dictionary_value_lossy(
    dictionary: &ffmpeg::DictionaryRef<'_>,
    key: &str,
) -> Option<String> {
    let key = CString::new(key).ok()?;
    // SAFETY: `dictionary` points to FFmpeg-owned metadata for the lifetime of this call.
    // Metadata bytes may be UTF-8 or legacy locale encodings such as GB18030.
    let value = unsafe {
        let entry = ffi::av_dict_get(dictionary.as_ptr(), key.as_ptr(), ptr::null_mut(), 0);
        if entry.is_null() || (*entry).value.is_null() {
            return None;
        }
        CStr::from_ptr((*entry).value).to_bytes()
    };
    let decoded = crate::app::media::lyrics::text_encoding::decode_bytes_to_text(value);
    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

fn normalize_metadata_value(value: &str) -> Option<String> {
    let trimmed = value.trim_matches(char::is_whitespace);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_media_kind(
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
) -> PlaybackMediaKind {
    let has_audio_stream = audio_stream_index.is_some();
    let has_primary_video_stream = input_ctx.streams().any(|stream| {
        stream.parameters().medium() == Type::Video
            && !stream.disposition().contains(Disposition::ATTACHED_PIC)
    });
    resolve_media_kind_from_stream_flags(has_audio_stream, has_primary_video_stream)
}

fn resolve_media_kind_from_stream_flags(
    has_audio_stream: bool,
    has_primary_video_stream: bool,
) -> PlaybackMediaKind {
    if has_audio_stream && !has_primary_video_stream {
        PlaybackMediaKind::Audio
    } else {
        PlaybackMediaKind::Video
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_media_kind_from_stream_flags;
    use crate::app::media::playback::dto::PlaybackMediaKind;

    #[test]
    fn classifies_audio_only_sources_as_audio() {
        assert_eq!(
            resolve_media_kind_from_stream_flags(true, false),
            PlaybackMediaKind::Audio,
        );
    }

    #[test]
    fn keeps_attached_cover_art_audio_in_audio_mode() {
        assert_eq!(
            resolve_media_kind_from_stream_flags(true, false),
            PlaybackMediaKind::Audio,
        );
    }

    #[test]
    fn classifies_primary_video_streams_as_video() {
        assert_eq!(
            resolve_media_kind_from_stream_flags(true, true),
            PlaybackMediaKind::Video,
        );
        assert_eq!(
            resolve_media_kind_from_stream_flags(false, true),
            PlaybackMediaKind::Video,
        );
    }
}
