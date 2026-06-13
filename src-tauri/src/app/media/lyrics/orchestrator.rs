use super::candidate::{dedupe_candidates, sort_candidates_by_provider_priority, LyricsCandidate};
use super::provider::{
    KugouProvider, LrcApiProvider, LrclibProvider, LyricsProvider, NeteaseProvider,
};
use super::track_signature::TrackSignature;
use crate::app::media::playback::session::player_settings::LyricsFetchSettings;

pub async fn fetch_all_candidates(
    client: &reqwest::Client,
    signature: &TrackSignature,
    settings: &LyricsFetchSettings,
) -> Vec<LyricsCandidate> {
    let mut candidates = Vec::new();
    if settings.providers.netease {
        candidates.extend(
            NeteaseProvider
                .fetch_candidates(client, signature)
                .await
                .unwrap_or_default(),
        );
    }
    if settings.providers.kugou {
        candidates.extend(
            KugouProvider
                .fetch_candidates(client, signature)
                .await
                .unwrap_or_default(),
        );
    }
    if settings.providers.lrclib {
        candidates.extend(
            LrclibProvider
                .fetch_candidates(client, signature)
                .await
                .unwrap_or_default(),
        );
    }
    if settings.providers.lrcapi {
        let provider = LrcApiProvider::new(Some(settings.lrc_api_base_url.as_str()));
        candidates.extend(
            provider
                .fetch_candidates(client, signature)
                .await
                .unwrap_or_default(),
        );
    }
    let mut candidates = dedupe_candidates(candidates);
    sort_candidates_by_provider_priority(&mut candidates);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::media::playback::session::player_settings::LyricsProviderSettings;

    #[test]
    fn disabled_lrclib_still_allows_lrcapi_config() {
        let settings = LyricsFetchSettings {
            auto_fetch_online_lyrics: true,
            providers: LyricsProviderSettings {
                lrclib: false,
                lrcapi: true,
                kugou: false,
                netease: false,
            },
            lrc_api_base_url: String::new(),
        };
        assert!(!settings.providers.lrclib);
        assert!(settings.providers.lrcapi);
    }
}
