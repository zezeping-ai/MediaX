pub(super) fn supports_adaptive_quality(source: &str) -> bool {
    let normalized = source.trim().to_ascii_lowercase();
    normalized.contains(".m3u8") || normalized.contains(".mpd")
}
