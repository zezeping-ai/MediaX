mod controls;
mod domains;
mod snapshot;
mod stream_runtime;

pub use crate::app::media::playback::rate::TimingControls;
pub use controls::AudioControls;
pub use domains::{MediaCacheState, MediaControlState, MediaRuntimeState, MediaSessionState};
pub use snapshot::{
    emit_snapshot, emit_snapshot_with_request_id, library, playback, snapshot_from_state,
};
pub use stream_runtime::{DecodeStreamHandles, StreamRuntimeState};

pub struct CacheRecorderSession {
    pub source: String,
    pub output_path: String,
    pub started_at_ms: u64,
    pub active: bool,
    pub fallback_transcoding: bool,
    pub error_message: Option<String>,
}

pub struct MediaState {
    pub session: MediaSessionState,
    pub runtime: MediaRuntimeState,
    pub controls: MediaControlState,
    pub cache: MediaCacheState,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            session: MediaSessionState::default(),
            runtime: MediaRuntimeState::default(),
            controls: MediaControlState::default(),
            cache: MediaCacheState::default(),
        }
    }
}
