pub(super) fn supports_adaptive_quality(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized.contains(".m3u8") || normalized.contains(".mpd")
}

pub(crate) fn supports_timeline_seek(source: &str) -> bool {
    !is_seek_limited_stream(source)
}

fn is_seek_limited_stream(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized.contains(".m3u8") || normalized.contains(".mpd")
}
