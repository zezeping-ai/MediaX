use crate::app::media::model::MediaLyricLine;

use super::candidate::LyricsCandidate;
use super::track_signature::TrackSignature;

#[derive(Debug)]
pub enum ProviderError {
    NotFound,
    Request(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "lyrics not found"),
            Self::Request(message) => write!(f, "lyrics request failed: {message}"),
        }
    }
}

pub struct ProviderResult {
    pub lines: Vec<MediaLyricLine>,
    pub synced: bool,
    pub provider_id: &'static str,
}

pub trait LyricsProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;

    async fn fetch(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<ProviderResult, ProviderError>;

    async fn fetch_candidates(
        &self,
        client: &reqwest::Client,
        signature: &TrackSignature,
    ) -> Result<Vec<LyricsCandidate>, ProviderError> {
        match self.fetch(client, signature).await {
            Ok(result) if !result.lines.is_empty() => Ok(vec![LyricsCandidate {
                id: format!("{}:0", self.id()),
                provider_id: self.id().to_string(),
                label: self.display_name().to_string(),
                synced: result.synced,
                preview: super::candidate::build_preview(&result.lines),
                lines: result.lines,
                track_name: Some(signature.track_name.clone()),
                artist_name: Some(signature.artist_name.clone()),
                duration_seconds: if signature.duration_seconds > 0.0 {
                    Some(signature.duration_seconds)
                } else {
                    None
                },
            }]),
            Ok(_) => Ok(Vec::new()),
            Err(ProviderError::NotFound) => Ok(Vec::new()),
            Err(err @ ProviderError::Request(_)) => Err(err),
        }
    }
}

pub fn user_agent() -> String {
    format!(
        "MediaX/{} (https://github.com/zezeping-ai/MediaX)",
        env!("CARGO_PKG_VERSION")
    )
}

mod lrclib;
mod lrcapi;
mod lrc_text;
mod kugou;
mod netease;

pub use lrclib::LrclibProvider;
pub use lrcapi::LrcApiProvider;
pub use kugou::KugouProvider;
pub use netease::NeteaseProvider;
