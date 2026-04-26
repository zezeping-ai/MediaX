mod controls;
mod snapshot;
mod stream_runtime;

use crate::app::media::library::MediaLibraryService;
use crate::app::media::playback::session::service::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

pub use controls::{AudioControls, TimingControls};
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

#[derive(Default)]
pub struct MediaState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
    pub stream: StreamRuntimeState,
    pub paused_seek_epoch: AtomicU32,
    pub preview_frame_epoch: AtomicU32,
    pub audio_controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
    pub cache_recorder: Mutex<Option<CacheRecorderSession>>,
}
