use reqwest::StatusCode;
use serde::Deserialize;

use super::{LyricsProvider, ProviderError, ProviderResult};
use crate::app::media::lyrics::candidate::{build_preview, truncate_preview, LyricsCandidate};
use crate::app::media::lyrics::lrc::parse_lrc_contents;
use crate::app::media::lyrics::plain::plain_text_to_timed_lines;
use crate::app::media::lyrics::track_signature::TrackSignature;

const DEFAULT_LRCAPI_BASE_URL: &str = "https://api.lrc.cx";
const LRCAPI_JSONAPI_LIMIT: usize = 10;

pub struct LrcApiProvider {
    base_url: String,
}

impl LrcApiProvider {
    pub fn new(base_url: Option<&str>) -> Self {
        let normalized = base_url
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_LRCAPI_BASE_URL)
            .trim_end_matches('/')
            .to_string();
        Self {
            base_url: normalized,
        }
    }
}

impl LyricsProvider for LrcApiProvider {
    fn id(&self) -> &'static str {
        "lrcapi"
    }

    fn display_name(&self) -> &'static str {
        "LrcApi"
    }

    async fn fetch(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<ProviderResult, ProviderError> {
        fetch_single(client, &self.base_url, signature)
            .await?
            .ok_or(ProviderError::NotFound)
    }

    async fn fetch_candidates(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        let mut candidates = Vec::new();
        if let Some(result) = fetch_single(client, &self.base_url, signature).await? {
            if let Some(candidate) = provider_result_to_candidate(
                result,
                "lrcapi:single",
                format_label("LrcApi", signature, None),
            ) {
                candidates.push(candidate);
            }
        }
        candidates.extend(fetch_jsonapi_candidates(client, &self.base_url, signature).await?);
        Ok(candidates)
    }
}

async fn fetch_single(
    client: &reqwest::Client,
    base_url: &str,
    signature: &TrackSignature,
) -> Result<Option<ProviderResult>, ProviderError> {
    let response = client
        .get(format!("{base_url}/lyrics"))
        .query(&[
            ("title", signature.track_name.as_str()),
            ("artist", signature.artist_name.as_str()),
            ("album", signature.album_name.as_str()),
        ])
        .timeout(std::time::Duration::from_millis(12_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Ok(None);
    }
    let body = response
        .text()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    map_lrc_text(&body, "lrcapi")
}

#[derive(Debug, Deserialize)]
struct LrcApiJsonApiRecord {
    #[serde(default)]
    album: Option<String>,
    #[serde(default)]
    lrc: Option<String>,
}

async fn fetch_jsonapi_candidates(
    client: &reqwest::Client,
    base_url: &str,
    signature: &TrackSignature,
) -> Result<Vec<LyricsCandidate>, ProviderError> {
    let response = client
        .get(format!("{base_url}/jsonapi"))
        .query(&[
            ("title", signature.track_name.as_str()),
            ("artist", signature.artist_name.as_str()),
            ("album", signature.album_name.as_str()),
        ])
        .timeout(std::time::Duration::from_millis(15_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    let records: Vec<LrcApiJsonApiRecord> = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    let mut candidates = Vec::new();
    for (index, record) in records.into_iter().take(LRCAPI_JSONAPI_LIMIT).enumerate() {
        let Some(text) = record.lrc.as_deref() else {
            continue;
        };
        let Some(result) = map_lrc_text(text, "lrcapi_jsonapi")? else {
            continue;
        };
        let label = format_label(
            "LrcApi 聚合",
            signature,
            Some((None, None, record.album.as_deref())),
        );
        if let Some(candidate) = provider_result_to_candidate(
            result,
            &format!("lrcapi:jsonapi:{index}"),
            label,
        ) {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

fn map_lrc_text(body: &str, provider_id: &'static str) -> Result<Option<ProviderResult>, ProviderError> {
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

fn provider_result_to_candidate(
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

fn format_label(
    provider: &str,
    signature: &TrackSignature,
    override_meta: Option<(Option<&str>, Option<&str>, Option<&str>)>,
) -> String {
    let (title, artist, album) = override_meta.unwrap_or((
        Some(signature.track_name.as_str()),
        Some(signature.artist_name.as_str()),
        Some(signature.album_name.as_str()),
    ));
    let title = title.unwrap_or(signature.track_name.as_str());
    let artist = artist.unwrap_or(signature.artist_name.as_str());
    let mut label = format!("{provider} · {artist} - {title}");
    if let Some(album) = album.filter(|value| !value.is_empty() && *value != "Unknown Album") {
        label.push_str(&format!(" · {album}"));
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
    #[ignore = "live lrcapi API probe for 青花瓷"]
    async fn live_fetches_qinghuaci() {
        let client = reqwest::Client::builder()
            .user_agent(super::super::user_agent())
            .build()
            .expect("client");
        let signature = qinghuaci_signature();
        let provider = LrcApiProvider::new(None);
        let candidates = provider
            .fetch_candidates(&client, &signature)
            .await
            .expect("fetch");
        assert!(
            candidates.iter().any(|candidate| contains_qinghuaci_text(&candidate.lines)),
            "expected 青花瓷 lyric candidates"
        );
    }
}
