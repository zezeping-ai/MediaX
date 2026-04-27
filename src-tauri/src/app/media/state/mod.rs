mod controls;
mod snapshot;
mod stream_runtime;

use crate::app::media::library::MediaLibraryService;
use crate::app::media::playback::session::service::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

pub use controls::{AudioControls, DebugControls, TimingControls};
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
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
    pub stream: StreamRuntimeState,
    pub paused_seek_epoch: AtomicU32,
    pub preview_frame_epoch: AtomicU32,
    pub audio_controls: Arc<AudioControls>,
    pub timing_controls: Arc<TimingControls>,
    pub debug_controls: Arc<DebugControls>,
    pub cache_recorder: Mutex<Option<CacheRecorderSession>>,
}

impl Default for MediaState {
    fn default() -> Self {
        Self {
            library: Mutex::default(),
            playback: Mutex::default(),
            stream: StreamRuntimeState::default(),
            paused_seek_epoch: AtomicU32::default(),
            preview_frame_epoch: AtomicU32::default(),
            audio_controls: Arc::new(AudioControls::default()),
            timing_controls: Arc::new(TimingControls::default()),
            debug_controls: Arc::new(DebugControls::default()),
            cache_recorder: Mutex::default(),
        }
    }
}
