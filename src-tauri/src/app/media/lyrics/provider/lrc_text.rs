use super::{ProviderError, ProviderResult};
use crate::app::media::lyrics::candidate::{build_preview, truncate_preview, LyricsCandidate};
use crate::app::media::lyrics::lrc::parse_lrc_contents;
use crate::app::media::lyrics::plain::plain_text_to_timed_lines;
use crate::app::media::lyrics::track_signature::TrackSignature;

pub fn map_lrc_text(body: &str, provider_id: &'static str) -> Result<Option<ProviderResult>, ProviderError> {
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("Lyrics not found.") {
        return Ok(None);
    }
    let normalized = trimmed
        .strip_prefix("[!text]")
        .map(str::trim)
        .unwrap_or(trimmed);
    let lines = parse_lrc_contents(normalized);
    if !lines.is_empty() {
        return Ok(Some(ProviderResult {
            lines,
            synced: true,
            provider_id,
        }));
    }
    let plain_lines = plain_text_to_timed_lines(normalized, 0.0);
    if plain_lines.is_empty() {
        return Ok(None);
    }
    Ok(Some(ProviderResult {
        lines: plain_lines,
        synced: false,
        provider_id,
    }))
}

pub fn provider_result_to_candidate(
    result: ProviderResult,
    id: &str,
    label: String,
) -> Option<LyricsCandidate> {
    if result.lines.is_empty() {
        return None;
    }
    Some(LyricsCandidate {
        id: id.to_string(),
        provider_id: result.provider_id.to_string(),
        label,
        synced: result.synced,
        preview: build_preview(&result.lines),
        lines: result.lines,
    })
}

pub fn duration_distance_seconds(target: f64, candidate_seconds: f64) -> f64 {
    if target <= 0.0 || candidate_seconds <= 0.0 {
        return 0.0;
    }
    (candidate_seconds - target).abs()
}

pub fn format_provider_label(
    provider: &str,
    signature: &TrackSignature,
    artist: Option<&str>,
    title: Option<&str>,
    album: Option<&str>,
) -> String {
    let artist = artist.unwrap_or(signature.artist_name.as_str());
    let title = title.unwrap_or(signature.track_name.as_str());
    let mut label = format!("{provider} · {artist} - {title}");
    if let Some(album) = album.filter(|value| !value.is_empty() && *value != "Unknown Album") {
        label.push_str(&format!(" · {album}"));
    }
    truncate_preview(&label, 72)
}

pub fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace() && !matches!(ch, '-' | '_' | '（' | '）' | '(' | ')' | '·'))
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn title_match_score(expected: &str, actual: &str) -> i32 {
    let expected = normalize_match_text(expected);
    let actual = normalize_match_text(actual);
    if expected.is_empty() || actual.is_empty() {
        return 0;
    }
    if expected == actual {
        return 100;
    }
    if actual.contains(expected.as_str()) || expected.contains(actual.as_str()) {
        return 80;
    }
    0
}

pub fn artist_match_score(expected: &str, actual: &str) -> i32 {
    let expected = normalize_match_text(expected);
    let actual = normalize_match_text(actual);
    if expected.is_empty() || actual.is_empty() || expected == "unknownartist" {
        return 0;
    }
    if actual.contains(expected.as_str()) || expected.contains(actual.as_str()) {
        return 40;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_match_prefers_exact_qinghuaci() {
        assert_eq!(title_match_score("青花瓷", "青花瓷"), 100);
        assert!(title_match_score("青花瓷", "青花瓷（正式版）") >= 80);
    }
}
