pub(super) fn should_tail_eof_for_source(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.contains(".m3u8") || (lower.ends_with(".mp4") && lower.contains("mediax-cache-"))
}

pub(super) fn is_network_source(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("rtsp://")
        || lower.starts_with("rtmp://")
        || lower.starts_with("mms://")
}

pub(super) fn is_realtime_source(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.contains(".m3u8") || lower.contains(".mpd")
}
