pub(super) fn should_tail_eof_for_source(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    lower.contains(".m3u8") || (lower.ends_with(".mp4") && lower.contains("mediax-cache-"))
}
