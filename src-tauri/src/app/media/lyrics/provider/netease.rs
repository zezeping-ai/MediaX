use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
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

const NETEASE_SEARCH_URL: &str = "https://music.163.com/api/search/get/web";
const NETEASE_LYRIC_URL: &str = "https://music.163.com/api/song/lyric";
const NETEASE_SEARCH_LIMIT: usize = 12;
const NETEASE_CANDIDATE_LIMIT: usize = 6;

pub struct NeteaseProvider;

impl LyricsProvider for NeteaseProvider {
    fn id(&self) -> &'static str {
        "netease"
    }

    fn display_name(&self) -> &'static str {
        "网易云音乐"
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
        fetch_lyric_by_id(client, record.id).await?.ok_or(ProviderError::NotFound)
    }

    async fn fetch_candidates(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        let records = search_records(client, signature).await?;
        let mut ranked: Vec<_> = records.iter().collect();
        ranked.sort_by_key(|record| std::cmp::Reverse(score_record(record, signature)));
        let mut candidates = Vec::new();
        for (index, record) in ranked.into_iter().take(NETEASE_CANDIDATE_LIMIT).enumerate() {
            if score_record(record, signature) <= 0 {
                continue;
            }
            let Some(result) = fetch_lyric_by_id(client, record.id).await? else {
                continue;
            };
            let label = format_provider_label(
                "网易云音乐",
                signature,
                Some(record.artist.as_str()),
                Some(record.title.as_str()),
                record.album.as_deref(),
            );
            if let Some(candidate) =
                provider_result_to_candidate(result, &format!("netease:{index}"), label)
            {
                candidates.push(candidate);
            }
        }
        Ok(candidates)
    }
}

#[derive(Debug, Clone)]
struct NeteaseSearchRecord {
    id: u64,
    title: String,
    artist: String,
    album: Option<String>,
    duration_seconds: f64,
}

#[derive(Debug, Deserialize)]
struct NeteaseSearchResponse {
    result: NeteaseSearchResult,
}

#[derive(Debug, Deserialize)]
struct NeteaseSearchResult {
    #[serde(default)]
    songs: Vec<NeteaseSongRecord>,
}

#[derive(Debug, Deserialize)]
struct NeteaseSongRecord {
    id: u64,
    name: String,
    #[serde(default)]
    artists: Vec<NeteaseArtistRecord>,
    album: NeteaseAlbumRecord,
    duration: i64,
}

#[derive(Debug, Deserialize)]
struct NeteaseArtistRecord {
    name: String,
}

#[derive(Debug, Deserialize)]
struct NeteaseAlbumRecord {
    name: String,
}

#[derive(Debug, Deserialize)]
struct NeteaseLyricResponse {
    #[serde(default)]
    lrc: Option<NeteaseLyricBody>,
}

#[derive(Debug, Deserialize)]
struct NeteaseLyricBody {
    #[serde(default)]
    lyric: Option<String>,
}

fn build_search_keyword(signature: &TrackSignature) -> String {
    let mut keyword = signature.track_name.clone();
    if !signature.artist_name.is_empty() && signature.artist_name != "Unknown Artist" {
        keyword.push(' ');
        keyword.push_str(&signature.artist_name);
    }
    if !signature.album_name.is_empty() && signature.album_name != "Unknown Album" {
        keyword.push(' ');
        keyword.push_str(&signature.album_name);
    }
    keyword
}

fn pick_best_record<'a>(
    records: &'a [NeteaseSearchRecord],
    signature: &TrackSignature,
) -> Option<&'a NeteaseSearchRecord> {
    records
        .iter()
        .max_by_key(|record| score_record(record, signature))
        .filter(|record| score_record(record, signature) > 0)
}

fn score_record(record: &NeteaseSearchRecord, signature: &TrackSignature) -> i32 {
    let mut score = title_match_score(&signature.track_name, &record.title)
        + artist_match_score(&signature.artist_name, &record.artist);
    if score <= 0 {
        return 0;
    }
    let distance = duration_distance_seconds(signature.duration_seconds, record.duration_seconds);
    if signature.duration_seconds > 0.0 && record.duration_seconds > 0.0 && distance > 12.0 {
        score -= 20;
    }
    score
}

async fn search_records(
    client: &reqwest::Client,
    signature: &TrackSignature,
) -> Result<Vec<NeteaseSearchRecord>, ProviderError> {
    let keyword = build_search_keyword(signature);
    let limit = NETEASE_SEARCH_LIMIT.to_string();
    let response = client
        .get(NETEASE_SEARCH_URL)
        .query(&[
            ("s", keyword.as_str()),
            ("type", "1"),
            ("limit", limit.as_str()),
        ])
        .headers(netease_headers())
        .timeout(std::time::Duration::from_millis(12_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(Vec::new());
    }
    let payload: NeteaseSearchResponse = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    Ok(payload
        .result
        .songs
        .into_iter()
        .map(|song| NeteaseSearchRecord {
            id: song.id,
            title: song.name,
            artist: song
                .artists
                .first()
                .map(|artist| artist.name.clone())
                .unwrap_or_default(),
            album: Some(song.album.name),
            duration_seconds: song.duration as f64 / 1000.0,
        })
        .collect())
}

async fn fetch_lyric_by_id(
    client: &reqwest::Client,
    song_id: u64,
) -> Result<Option<ProviderResult>, ProviderError> {
    let id = song_id.to_string();
    let response = client
        .get(NETEASE_LYRIC_URL)
        .query(&[("id", id.as_str()), ("lv", "1")])
        .headers(netease_headers())
        .timeout(std::time::Duration::from_millis(12_000))
        .send()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let payload: NeteaseLyricResponse = response
        .json()
        .await
        .map_err(|err| ProviderError::Request(err.to_string()))?;
    let Some(text) = payload.lrc.and_then(|body| body.lyric) else {
        return Ok(None);
    };
    map_lrc_text(&text, "netease")
}

fn netease_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        ),
    );
    headers.insert(REFERER, HeaderValue::from_static("https://music.163.com/"));
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
        let record = NeteaseSearchRecord {
            id: 185811,
            title: "青花瓷".to_string(),
            artist: "周杰伦".to_string(),
            album: Some("我很忙".to_string()),
            duration_seconds: 239.0,
        };
        assert!(score_record(&record, &qinghuaci_signature()) >= 100);
    }

    #[tokio::test]
    #[ignore = "live netease API probe for 青花瓷"]
    async fn live_fetches_qinghuaci() {
        let client = reqwest::Client::builder()
            .user_agent(user_agent())
            .build()
            .expect("client");
        let signature = qinghuaci_signature();
        let result = NeteaseProvider.fetch(&client, &signature).await.expect("fetch");
        assert!(result.synced);
        assert!(
            result.lines.iter().any(|line| line.text.contains("天青色等烟雨")),
            "expected 青花瓷 lyric line"
        );
    }
}
