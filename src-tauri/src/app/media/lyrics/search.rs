use crate::app::media::lyrics::candidate::LyricsCandidate;
use crate::app::media::lyrics::format_lrc_contents;
use crate::app::media::lyrics::orchestrator::fetch_all_candidates;
use crate::app::media::lyrics::provider::user_agent;
use crate::app::media::lyrics::track_signature::TrackSignature;
use crate::app::media::model::LyricsSearchHit;
use crate::app::media::playback::session::player_settings::lyrics_fetch_settings;

pub async fn search_lyrics_hits(
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    duration_seconds: f64,
) -> Result<Vec<LyricsSearchHit>, String> {
    let track_name = title.trim();
    let artist_name = artist.unwrap_or("").trim();
    if track_name.is_empty() && artist_name.is_empty() {
        return Err("请至少输入歌曲名称或作者".to_string());
    }

    let signature = TrackSignature {
        track_name: if track_name.is_empty() {
            "Unknown Track".to_string()
        } else {
            track_name.to_string()
        },
        artist_name: if artist_name.is_empty() {
            "Unknown Artist".to_string()
        } else {
            artist_name.to_string()
        },
        album_name: album
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Unknown Album")
            .to_string(),
        duration_seconds: duration_seconds.max(0.0),
    };

    let client = reqwest::Client::builder()
        .user_agent(user_agent())
        .build()
        .map_err(|err| format!("创建网络客户端失败: {err}"))?;
    let candidates = fetch_all_candidates(&client, &signature, &lyrics_fetch_settings()).await;
    Ok(candidates
        .into_iter()
        .map(|candidate| candidate_to_hit(candidate, &signature))
        .collect())
}

fn candidate_to_hit(candidate: LyricsCandidate, signature: &TrackSignature) -> LyricsSearchHit {
    LyricsSearchHit {
        id: candidate.id,
        provider_id: candidate.provider_id,
        title: candidate
            .track_name
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| signature.track_name.clone()),
        artist: candidate
            .artist_name
            .clone()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| signature.artist_name.clone()),
        album: if signature.album_name.is_empty() || signature.album_name == "Unknown Album" {
            None
        } else {
            Some(signature.album_name.clone())
        },
        duration_seconds: candidate.duration_seconds,
        synced: candidate.synced,
        preview: candidate.preview,
        lyrics_lrc: format_lrc_contents(&candidate.lines),
    }
}
