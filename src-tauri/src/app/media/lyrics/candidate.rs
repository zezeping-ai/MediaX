use crate::app::media::model::{LyricsCandidateSummary, MediaLyricLine};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsCandidate {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub synced: bool,
    pub preview: String,
    pub lines: Vec<MediaLyricLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artist_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
}

impl LyricsCandidate {
    pub fn summary(&self) -> LyricsCandidateSummary {
        LyricsCandidateSummary {
            id: self.id.clone(),
            provider_id: self.provider_id.clone(),
            label: self.label.clone(),
            synced: self.synced,
            preview: self.preview.clone(),
            track_name: self.track_name.clone(),
            artist_name: self.artist_name.clone(),
            duration_seconds: self.duration_seconds,
        }
    }
}

pub fn build_preview(lines: &[MediaLyricLine]) -> String {
    for line in lines {
        let text = line.text.trim();
        if text.is_empty() {
            continue;
        }
        if text.starts_with('[') && text.ends_with(']') {
            continue;
        }
        return truncate_preview(text, 48);
    }
    String::new()
}

pub fn truncate_preview(value: &str, max_chars: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    chars
        .into_iter()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        + "…"
}

pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch,
            '\u{3400}'..='\u{4DBF}'
                | '\u{4E00}'..='\u{9FFF}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{3040}'..='\u{30FF}'
                | '\u{AC00}'..='\u{D7AF}'
        )
    })
}

pub fn pick_default_candidate_id(
    candidates: &[LyricsCandidate],
    saved_candidate_id: Option<&str>,
    prefer_cjk: bool,
    duration_seconds: f64,
) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    if let Some(saved_id) = saved_candidate_id {
        if candidates.iter().any(|candidate| candidate.id == saved_id) {
            return Some(saved_id.to_string());
        }
    }

    let mut ranked: Vec<(i32, &LyricsCandidate)> = candidates
        .iter()
        .map(|candidate| (score_candidate(candidate, prefer_cjk, duration_seconds), candidate))
        .collect();
    ranked.sort_by(|left, right| right.0.cmp(&left.0));
    ranked.first().map(|(_, candidate)| candidate.id.clone())
}

fn score_candidate(candidate: &LyricsCandidate, prefer_cjk: bool, duration_seconds: f64) -> i32 {
    let mut score = 0;
    if candidate.synced {
        score += 20;
    }
    if prefer_cjk && contains_cjk(&candidate.preview) {
        score += 30;
    }
    if prefer_cjk && contains_cjk(&candidate.label) {
        score += 10;
    }
    score += provider_priority(&candidate.provider_id);
    if duration_seconds > 0.0 {
        if let Some(duration_hint) = parse_duration_hint(&candidate.label) {
            let distance = (duration_hint - duration_seconds).abs();
            if distance <= 3.0 {
                score += 25;
            } else if distance <= 8.0 {
                score += 12;
            } else if distance <= 20.0 {
                score += 4;
            }
        }
    }
    if !candidate.preview.is_empty() {
        score += 2;
    }
    score
}

fn parse_duration_hint(label: &str) -> Option<f64> {
    let open = label.rfind('(')?;
    let close = label.rfind(')')?;
    if close <= open {
        return None;
    }
    let value = label.get(open + 1..close)?.trim();
    let seconds = value.strip_suffix('s')?.trim().parse::<f64>().ok()?;
    Some(seconds)
}

pub fn local_lyrics_candidate(lines: &[MediaLyricLine], source: Option<&str>) -> LyricsCandidate {
    let provider_id = source.unwrap_or("local").to_string();
    let label = match source {
        Some("sidecar") => "本地 LRC",
        Some("embedded") => "内嵌歌词",
        _ => "本地歌词",
    };
    LyricsCandidate {
        id: format!("local:{provider_id}"),
        provider_id: provider_id.clone(),
        label: format!("{label} · {}", build_preview(lines)),
        synced: lines.iter().any(|line| line.time_seconds > 0.0),
        preview: build_preview(lines),
        lines: lines.to_vec(),
        track_name: None,
        artist_name: None,
        duration_seconds: None,
    }
}

pub fn merge_candidates(
    seed: Vec<LyricsCandidate>,
    online: Vec<LyricsCandidate>,
) -> Vec<LyricsCandidate> {
    let mut merged = seed;
    merged.extend(online);
    let mut merged = dedupe_candidates(merged);
    sort_candidates_by_provider_priority(&mut merged);
    merged
}

pub fn dedupe_candidates(mut candidates: Vec<LyricsCandidate>) -> Vec<LyricsCandidate> {
    let mut unique: Vec<LyricsCandidate> = Vec::new();
    'outer: for candidate in candidates.drain(..) {
        if candidate.lines.is_empty() {
            continue;
        }
        let fingerprint = candidate_fingerprint(&candidate.lines);
        for existing in &unique {
            if candidate_fingerprint(&existing.lines) == fingerprint {
                continue 'outer;
            }
        }
        unique.push(candidate);
    }
    unique
}

/// 歌词源优先级：本地 > 网易云 > 酷狗 > 其他在线源
pub fn provider_priority(provider_id: &str) -> i32 {
    match provider_id {
        "embedded" => 65,
        "sidecar" => 55,
        "netease" => 40,
        "kugou" => 30,
        "lrclib" => 8,
        "lrcapi_jsonapi" => 6,
        "lrcapi" => 4,
        "cache" => 3,
        _ => 0,
    }
}

pub fn sort_candidates_by_provider_priority(candidates: &mut [LyricsCandidate]) {
    candidates.sort_by(|left, right| {
        provider_priority(&right.provider_id)
            .cmp(&provider_priority(&left.provider_id))
            .then_with(|| left.label.cmp(&right.label))
    });
}

fn candidate_fingerprint(lines: &[MediaLyricLine]) -> String {
    lines
        .iter()
        .take(6)
        .map(|line| line.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_saved_candidate_when_available() {
        let candidates = vec![
            LyricsCandidate {
                id: "a".to_string(),
                provider_id: "lrclib".to_string(),
                label: "A".to_string(),
                synced: true,
                preview: "中文".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "中文".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
            LyricsCandidate {
                id: "b".to_string(),
                provider_id: "netease".to_string(),
                label: "B".to_string(),
                synced: false,
                preview: "English".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "English".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
        ];
        assert_eq!(
            pick_default_candidate_id(&candidates, Some("b"), true, 240.0),
            Some("b".to_string())
        );
    }

    #[test]
    fn merge_candidates_keeps_seed_when_online_empty() {
        let seed = vec![LyricsCandidate {
            id: "cache:0".to_string(),
            provider_id: "cache".to_string(),
            label: "Cache".to_string(),
            synced: true,
            preview: "cached".to_string(),
            lines: vec![MediaLyricLine {
                time_seconds: 0.0,
                text: "cached".to_string(),
            }],
            track_name: None,
            artist_name: None,
            duration_seconds: None,
        }];
        let merged = merge_candidates(seed, Vec::new());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "cache:0");
    }

    #[test]
    fn prefers_netease_over_lrclib_when_scores_otherwise_close() {
        let candidates = vec![
            LyricsCandidate {
                id: "lrclib:0".to_string(),
                provider_id: "lrclib".to_string(),
                label: "LRCLIB".to_string(),
                synced: true,
                preview: "歌词".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "歌词".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
            LyricsCandidate {
                id: "netease:0".to_string(),
                provider_id: "netease".to_string(),
                label: "Netease".to_string(),
                synced: true,
                preview: "歌词".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 1.0,
                    text: "歌词二".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
        ];
        assert_eq!(
            pick_default_candidate_id(&candidates, None, true, 240.0),
            Some("netease:0".to_string())
        );
    }

    #[test]
    fn prefers_kugou_over_lrclib() {
        let candidates = vec![
            LyricsCandidate {
                id: "lrclib:0".to_string(),
                provider_id: "lrclib".to_string(),
                label: "LRCLIB".to_string(),
                synced: true,
                preview: "歌词".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "歌词".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
            LyricsCandidate {
                id: "kugou:0".to_string(),
                provider_id: "kugou".to_string(),
                label: "Kugou".to_string(),
                synced: true,
                preview: "歌词".to_string(),
                lines: vec![MediaLyricLine {
                    time_seconds: 1.0,
                    text: "歌词二".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
        ];
        assert_eq!(
            pick_default_candidate_id(&candidates, None, true, 240.0),
            Some("kugou:0".to_string())
        );
    }

    #[test]
    fn sort_candidates_by_provider_priority_orders_netease_before_kugou_before_lrclib() {
        let mut candidates = vec![
            LyricsCandidate {
                id: "lrclib:0".to_string(),
                provider_id: "lrclib".to_string(),
                label: "C".to_string(),
                synced: false,
                preview: String::new(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "c".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
            LyricsCandidate {
                id: "kugou:0".to_string(),
                provider_id: "kugou".to_string(),
                label: "B".to_string(),
                synced: false,
                preview: String::new(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "b".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
            LyricsCandidate {
                id: "netease:0".to_string(),
                provider_id: "netease".to_string(),
                label: "A".to_string(),
                synced: false,
                preview: String::new(),
                lines: vec![MediaLyricLine {
                    time_seconds: 0.0,
                    text: "a".to_string(),
                }],
                track_name: None,
                artist_name: None,
                duration_seconds: None,
            },
        ];
        sort_candidates_by_provider_priority(&mut candidates);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.provider_id.as_str())
                .collect::<Vec<_>>(),
            vec!["netease", "kugou", "lrclib"]
        );
    }
}
