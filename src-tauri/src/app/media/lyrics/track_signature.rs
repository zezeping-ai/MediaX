use std::path::Path;

use super::text_encoding::decode_os_str;

#[derive(Debug, Clone)]
pub struct TrackSignature {
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration_seconds: f64,
}

impl TrackSignature {
    /// Identity signature for cache and playback state. Prefers embedded tags, then filename.
    pub fn from_metadata(
        source: &str,
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        duration_seconds: f64,
    ) -> Self {
        let parsed = parse_filename_signature(source);
        let track_name = normalize_field(title).or(parsed.title).unwrap_or_else(|| {
            file_stem_label(source).unwrap_or_else(|| "Unknown Track".to_string())
        });
        let artist_name = normalize_field(artist)
            .or(parsed.artist)
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let album_name = normalize_field(album).unwrap_or_else(|| "Unknown Album".to_string());
        Self {
            track_name,
            artist_name,
            album_name,
            duration_seconds: duration_seconds.max(0.0),
        }
    }

    /// Online lyrics search uses embedded audio metadata only.
    pub fn for_online_search(
        title: Option<&str>,
        artist: Option<&str>,
        album: Option<&str>,
        duration_seconds: f64,
    ) -> Option<Self> {
        if normalize_field(title).is_none() && normalize_field(artist).is_none() {
            return None;
        }
        Some(Self {
            track_name: normalize_field(title).unwrap_or_else(|| "Unknown Track".to_string()),
            artist_name: normalize_field(artist).unwrap_or_else(|| "Unknown Artist".to_string()),
            album_name: normalize_field(album).unwrap_or_else(|| "Unknown Album".to_string()),
            duration_seconds: duration_seconds.max(0.0),
        })
    }
}

#[derive(Default)]
struct FilenameSignature {
    title: Option<String>,
    artist: Option<String>,
}

fn parse_filename_signature(source: &str) -> FilenameSignature {
    let Some(stem) = file_stem_label(source) else {
        return FilenameSignature::default();
    };
    if let Some((left, right)) = split_artist_title(&stem) {
        return FilenameSignature {
            artist: Some(left.to_string()),
            title: Some(right.to_string()),
        };
    }
    FilenameSignature {
        title: Some(stem),
        ..FilenameSignature::default()
    }
}

fn file_stem_label(source: &str) -> Option<String> {
    Path::new(source)
        .file_stem()
        .map(decode_os_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn split_artist_title(value: &str) -> Option<(&str, &str)> {
    for separator in [" - ", " – ", " — "] {
        if let Some((left, right)) = value.split_once(separator) {
            let artist = left.trim();
            let title = right.trim();
            if !artist.is_empty() && !title.is_empty() {
                return Some((artist, title));
            }
        }
    }
    None
}

fn normalize_field(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TrackSignature;

    #[test]
    fn online_search_requires_embedded_metadata() {
        assert!(TrackSignature::for_online_search(None, None, None, 240.0).is_none());
        assert!(TrackSignature::for_online_search(Some("如果云知道"), None, None, 240.0).is_some());
    }

    #[test]
    fn online_search_uses_tag_fields_only() {
        let signature = TrackSignature::for_online_search(
            Some("如果云知道"),
            Some("许茹芸"),
            Some("专辑"),
            240.0,
        )
        .expect("signature");
        assert_eq!(signature.track_name, "如果云知道");
        assert_eq!(signature.artist_name, "许茹芸");
        assert_eq!(signature.album_name, "专辑");
    }

    #[test]
    fn cache_signature_still_falls_back_to_filename() {
        let signature = TrackSignature::from_metadata(
            "/music/Artist - Title.mp3",
            None,
            None,
            None,
            240.0,
        );
        assert_eq!(signature.track_name, "Title");
        assert_eq!(signature.artist_name, "Artist");
    }
}
