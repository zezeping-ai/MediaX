use reqwest::StatusCode;
use serde::Deserialize;

use super::{LyricsProvider, ProviderError, ProviderResult};
use crate::app::media::lyrics::candidate::{truncate_preview, LyricsCandidate};
use crate::app::media::lyrics::lrc::parse_lrc_contents;
use crate::app::media::lyrics::plain::plain_text_to_timed_lines;
use crate::app::media::lyrics::provider::lrc_text::provider_result_to_candidate;
use crate::app::media::lyrics::track_signature::TrackSignature;

const LRCLIB_BASE_URL: &str = "https://lrclib.net";
const LRCLIB_SEARCH_LIMIT: usize = 12;

type CandidateLabelOverride<'a> = (
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<f64>,
);

pub struct LrclibProvider;

impl LyricsProvider for LrclibProvider {
    fn id(&self) -> &'static str {
        "lrclib"
    }

    fn display_name(&self) -> &'static str {
        "LRCLIB"
    }

    async fn fetch(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<ProviderResult, ProviderError> {
        if let Some(result) = request_record(client, "/api/get-cached", signature, false).await? {
            return Ok(result);
        }
        if let Some(result) = search_best_match(client, signature).await? {
            return Ok(result);
        }
        request_record(client, "/api/get", signature, true)
            .await?
            .ok_or(ProviderError::NotFound)
    }

    async fn fetch_candidates(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        let mut candidates = Vec::new();
        if let Some(result) = request_record(client, "/api/get-cached", signature, false).await? {
            if let Some(candidate) = provider_result_to_candidate(
                result,
                "lrclib:cached",
                format_label("LRCLIB", signature, None),
                Some(signature.track_name.clone()),
                Some(signature.artist_name.clone()),
                if signature.duration_seconds > 0.0 {
                    Some(signature.duration_seconds)
                } else {
                    None
                },
            ) {
                candidates.push(candidate);
            }
        }
        candidates.extend(search_candidates(client, signature).await?);
        if let Some(result) = request_record(client, "/api/get", signature, true).await? {
            if let Some(candidate) = provider_result_to_candidate(
                result,
                "lrclib:get",
                format_label("LRCLIB", signature, None),
                Some(signature.track_name.clone()),
                Some(signature.artist_name.clone()),
                if signature.duration_seconds > 0.0 {
                    Some(signature.duration_seconds)
                } else {
                    None
                },
            ) {
                candidates.push(candidate);
            }
        }
        Ok(candidates)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LrclibRecord {
    #[serde(default)]
    track_name: Option<String>,
    #[serde(default)]
    artist_name: Option<String>,
    #[serde(default)]
    album_name: Option<String>,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
}

async fn request_record(
    client: &reqwest::Client,
    path: &str,
    signature: &TrackSignature,
    slow: bool,
) -> Result<Option<ProviderResult>, ProviderError> {
    let timeout = if slow { 15_000 } else { 8_000 };
    let response = client
        .get(format!("{LRCLIB_BASE_URL}{path}"))
        .query(&[
            ("track_name", signature.track_name.as_str()),
            ("artist_name", signature.artist_name.as_str()),
            ("album_name", signature.album_name.as_str()),
            ("duration", &signature.duration_seconds.round().to_string()),
        ])
        .timeout(std::time::Duration::from_millis(timeout))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(ProviderError::Request(format!(
            "lrclib status {}",
            response.status()
        )));
    }
    let record: LrclibRecord = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    Ok(map_record(record, signature.duration_seconds))
}

async fn search_best_match(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Option<ProviderResult>, ProviderError> {
    let records = fetch_search_records(client, signature).await?;
    let best = records
        .into_iter()
        .filter(|record| !record.instrumental)
        .min_by(|left, right| {
            duration_distance(signature.duration_seconds, left.duration)
                .partial_cmp(&duration_distance(signature.duration_seconds, right.duration))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    Ok(best.and_then(|record| map_record(record, signature.duration_seconds)))
}

async fn search_candidates(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Vec<LyricsCandidate>, ProviderError> {
    let records = fetch_search_records(client, signature).await?;
    let mut candidates = Vec::new();
    for (index, record) in records
        .into_iter()
        .filter(|record| !record.instrumental)
        .take(LRCLIB_SEARCH_LIMIT)
        .enumerate()
    {
        let label = format_label(
            "LRCLIB",
            signature,
            Some((
                record.track_name.as_deref(),
                record.artist_name.as_deref(),
                record.album_name.as_deref(),
                record.duration,
            )),
        );
        if let Some(result) = map_record(record.clone(), signature.duration_seconds) {
            if let Some(candidate) =
                provider_result_to_candidate(
                    result,
                    &format!("lrclib:search:{index}"),
                    label,
                    record.track_name.clone(),
                    record.artist_name.clone(),
                    record.duration,
                )
            {
                candidates.push(candidate);
            }
        }
    }
    Ok(candidates)
}

async fn fetch_search_records(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Vec<LrclibRecord>, ProviderError> {
    let response = client
        .get(format!("{LRCLIB_BASE_URL}/api/search"))
        .query(&[
            ("track_name", signature.track_name.as_str()),
            ("artist_name", signature.artist_name.as_str()),
        ])
        .timeout(std::time::Duration::from_millis(8_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))
}

fn duration_distance(target: f64, candidate: Option<f64>) -> f64 {
    candidate
        .map(|value| (value - target).abs())
        .unwrap_or(f64::MAX)
}

fn map_record(record: LrclibRecord, duration_seconds: f64) -> Option<ProviderResult> {
    if record.instrumental {
        return None;
    }
    if let Some(synced) = record.synced_lyrics.as_deref() {
        let lines = parse_lrc_contents(synced);
        if !lines.is_empty() {
            return Some(ProviderResult {
                lines,
                synced: true,
                provider_id: "lrclib",
            });
        }
    }
    if let Some(plain) = record.plain_lyrics.as_deref() {
        let lines = plain_text_to_timed_lines(plain, duration_seconds);
        if !lines.is_empty() {
            return Some(ProviderResult {
                lines,
                synced: false,
                provider_id: "lrclib",
            });
        }
    }
    None
}

fn format_label(
    provider: &str,
    signature: &TrackSignature,
    override_meta: Option<CandidateLabelOverride<'_>>,
) -> String {
    let (track, artist, album, duration) = override_meta.unwrap_or((
        Some(signature.track_name.as_str()),
        Some(signature.artist_name.as_str()),
        Some(signature.album_name.as_str()),
        if signature.duration_seconds > 0.0 {
            Some(signature.duration_seconds)
        } else {
            None
        },
    ));
    let track = track.unwrap_or(signature.track_name.as_str());
    let artist = artist.unwrap_or(signature.artist_name.as_str());
    let mut label = format!("{provider} · {artist} - {track}");
    if let Some(album) = album.filter(|value| !value.is_empty() && *value != signature.album_name) {
        label.push_str(&format!(" · {album}"));
    }
    if let Some(duration) = duration {
        label.push_str(&format!(" ({:.0}s)", duration));
    }
    truncate_preview(&label, 72)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::LyricsProvider;
    use crate::app::media::lyrics::track_signature::TrackSignature;
    use crate::app::media::model::MediaLyricLine;

    fn qinghuaci_signature() -> TrackSignature {
        TrackSignature {
            track_name: "青花瓷".to_string(),
            artist_name: "周杰伦".to_string(),
            album_name: "我很忙".to_string(),
            duration_seconds: 239.0,
        }
    }

    fn contains_qinghuaci_text(lines: &[MediaLyricLine]) -> bool {
        let joined: String = lines.iter().map(|line| line.text.as_str()).collect();
        joined.contains("素胚") || joined.contains("天青") || joined.contains("青花")
    }

    #[tokio::test]
    #[ignore = "live lrclib API probe for 青花瓷"]
    async fn live_fetches_qinghuaci() {
        let client = reqwest::Client::builder()
            .user_agent(super::super::user_agent())
            .build()
            .expect("client");
        let signature = qinghuaci_signature();
        let candidates = LrclibProvider
            .fetch_candidates(&client, &signature)
            .await
            .expect("fetch");
        assert!(
            candidates.iter().any(|candidate| contains_qinghuaci_text(&candidate.lines)),
            "expected 青花瓷 lyric candidates"
        );
    }
}
