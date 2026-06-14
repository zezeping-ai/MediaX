use base64::Engine;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

use super::lrc_text::{
    artist_match_score, duration_distance_seconds, format_provider_label, map_lrc_text,
    provider_result_to_candidate, title_match_score,
};
use super::{LyricsProvider, ProviderError, ProviderResult};
#[cfg(test)]
use super::user_agent;
use crate::app::media::lyrics::candidate::LyricsCandidate;
use crate::app::media::lyrics::track_signature::TrackSignature;

const KUGOU_SEARCH_URL: &str = "http://krcs.kugou.com/search";
const KUGOU_DOWNLOAD_URL: &str = "https://lyrics.kugou.com/download";
const KUGOU_CANDIDATE_LIMIT: usize = 6;

pub struct KugouProvider;

impl LyricsProvider for KugouProvider {
    fn id(&self) -> &'static str {
        "kugou"
    }

    fn display_name(&self) -> &'static str {
        "酷狗音乐"
    }

    async fn fetch(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<ProviderResult, ProviderError> {
        let records = search_records(client, signature).await?;
        let Some(record) = pick_best_record(&records, signature) else {
            return Err(ProviderError::NotFound);
        };
        download_record(client, record, signature)
            .await?
            .ok_or(ProviderError::NotFound)
    }

    async fn fetch_candidates(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        search_candidates(client, signature).await
    }
}

#[derive(Debug, Clone, Deserialize)]
struct KugouSearchRecord {
    id: String,
    accesskey: String,
    #[serde(default)]
    singer: Option<String>,
    #[serde(default)]
    song: Option<String>,
    #[serde(default)]
    duration: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct KugouSearchResponse {
    #[serde(default)]
    candidates: Vec<KugouSearchRecord>,
}

#[derive(Debug, Deserialize)]
struct KugouDownloadResponse {
    content: String,
}

fn build_search_keyword(signature: &TrackSignature) -> String {
    if signature.artist_name.is_empty() || signature.artist_name == "Unknown Artist" {
        signature.track_name.clone()
    } else {
        format!("{} - {}", signature.artist_name, signature.track_name)
    }
}

fn pick_best_record<'a>(
    records: &'a [KugouSearchRecord],
    signature: &TrackSignature,
) -> Option<&'a KugouSearchRecord> {
    records
        .iter()
        .max_by_key(|record| score_record(record, signature))
        .filter(|record| score_record(record, signature) > 0)
}

fn score_record(record: &KugouSearchRecord, signature: &TrackSignature) -> i32 {
    let title = record.song.as_deref().unwrap_or_default();
    let artist = record.singer.as_deref().unwrap_or_default();
    let mut score = title_match_score(&signature.track_name, title)
        + artist_match_score(&signature.artist_name, artist);
    if score <= 0 {
        return 0;
    }
    let candidate_seconds = record.duration.unwrap_or(0) as f64 / 1000.0;
    let distance = duration_distance_seconds(signature.duration_seconds, candidate_seconds);
    if signature.duration_seconds > 0.0 && candidate_seconds > 0.0 && distance > 12.0 {
        score -= 20;
    }
    score
}

async fn search_records(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Vec<KugouSearchRecord>, ProviderError> {
    let keyword = build_search_keyword(signature);
    let duration_ms = if signature.duration_seconds > 0.0 {
        (signature.duration_seconds * 1000.0).round().to_string()
    } else {
        String::new()
    };
    let response = client
        .get(KUGOU_SEARCH_URL)
        .query(&[
            ("ver", "1"),
            ("man", "yes"),
            ("client", "pc"),
            ("keyword", keyword.as_str()),
            ("duration", duration_ms.as_str()),
            ("hash", ""),
        ])
        .headers(kugou_headers())
        .timeout(std::time::Duration::from_millis(12_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    let payload: KugouSearchResponse = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    Ok(payload.candidates)
}

async fn search_candidates(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Vec<LyricsCandidate>, ProviderError> {
    let records = search_records(client, signature).await?;
    let mut ranked: Vec<_> = records.iter().collect();
    ranked.sort_by_key(|record| std::cmp::Reverse(score_record(record, signature)));
    let mut candidates = Vec::new();
    for (index, record) in ranked.into_iter().take(KUGOU_CANDIDATE_LIMIT).enumerate() {
        if score_record(record, signature) <= 0 {
            continue;
        }
        let Some(result) = download_record(client, record, signature).await? else {
            continue;
        };
        let label = format_provider_label(
            "酷狗音乐",
            signature,
            record.singer.as_deref(),
            record.song.as_deref(),
            None,
        );
        if let Some(candidate) = provider_result_to_candidate(
            result,
            &format!("kugou:{index}"),
            label,
            record.song.clone(),
            record.singer.clone(),
            record.duration.map(|value| value as f64 / 1000.0),
        ) {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

async fn download_record(
    client: &reqwest::Client,
    record: &KugouSearchRecord,
    signature: &TrackSignature,
) -> Result<Option<ProviderResult>, ProviderError> {
    let response = client
        .get(KUGOU_DOWNLOAD_URL)
        .query(&[
            ("ver", "1"),
            ("client", "pc"),
            ("id", record.id.as_str()),
            ("accesskey", record.accesskey.as_str()),
            ("fmt", "lrc"),
            ("charset", "utf8"),
        ])
        .headers(kugou_headers())
        .timeout(std::time::Duration::from_millis(12_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let payload: KugouDownloadResponse = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload.content.trim())
        .map_err(|err| ProviderError::Request(format!("kugou lyric decode failed: {err}")))?;
    let text = String::from_utf8(decoded)
        .map_err(|err| ProviderError::Request(format!("kugou lyric utf8 failed: {err}")))?;
    let _ = signature;
    map_lrc_text(&text, "kugou")
}

fn kugou_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::media::lyrics::track_signature::TrackSignature;

    fn qinghuaci_signature() -> TrackSignature {
        TrackSignature {
            track_name: "青花瓷".to_string(),
            artist_name: "周杰伦".to_string(),
            album_name: "我很忙".to_string(),
            duration_seconds: 239.0,
        }
    }

    #[test]
    fn scores_qinghuaci_record_high() {
        let record = KugouSearchRecord {
            id: "1".to_string(),
            accesskey: "key".to_string(),
            singer: Some("周杰伦".to_string()),
            song: Some("青花瓷".to_string()),
            duration: Some(239_000),
        };
        assert!(score_record(&record, &qinghuaci_signature()) >= 100);
    }

    #[tokio::test]
    #[ignore = "live kugou API probe for 青花瓷"]
    async fn live_fetches_qinghuaci() {
        let client = reqwest::Client::builder()
            .user_agent(user_agent())
            .build()
            .expect("client");
        let signature = qinghuaci_signature();
        let result = KugouProvider.fetch(&client, &signature).await.expect("fetch");
        assert!(result.synced);
        assert!(
            result.lines.iter().any(|line| line.text.contains("天青色等烟雨")),
            "expected 青花瓷 lyric line"
        );
    }
}
