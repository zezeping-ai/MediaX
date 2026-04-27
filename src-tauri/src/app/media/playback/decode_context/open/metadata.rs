use super::cover_art::extract_cover_frame;
use super::lyrics::load_source_lyrics;
use crate::app::media::model::MediaLyricLine;
use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::render::renderer::VideoFrame;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use ffmpeg_next::format::stream::Disposition;
use ffmpeg_next::media::Type;

const TITLE_METADATA_KEYS: &[&str] = &["title", "TITLE"];
const ARTIST_METADATA_KEYS: &[&str] = &["artist", "ARTIST", "album_artist", "ALBUMARTIST"];
const ALBUM_METADATA_KEYS: &[&str] = &["album", "ALBUM"];

pub(super) struct SourceMetadata {
    pub(super) media_kind: PlaybackMediaKind,
    pub(super) has_cover_art: bool,
    pub(super) cover_frame: Option<VideoFrame>,
    pub(super) title: Option<String>,
    pub(super) artist: Option<String>,
    pub(super) album: Option<String>,
    pub(super) lyrics: Vec<MediaLyricLine>,
}

pub(super) fn build_source_metadata(
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
    let lyrics = load_source_lyrics(source, input_ctx, audio_stream_index)?;
    let cover_frame = extract_cover_frame(input_ctx, cover_stream_index);

    Ok(SourceMetadata {
        media_kind: resolve_media_kind(input_ctx, audio_stream_index),
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

fn resolve_media_kind(
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
) -> PlaybackMediaKind {
    let has_audio_stream = audio_stream_index.is_some();
    let has_primary_video_stream = input_ctx
        .streams()
        .any(|stream| {
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
