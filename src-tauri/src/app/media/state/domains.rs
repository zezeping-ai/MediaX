use super::controls::{AudioControls, DebugControls};
use super::stream_runtime::StreamRuntimeState;
use super::CacheRecorderSession;
use crate::app::media::library::MediaLibraryService;
use crate::app::media::playback::rate::TimingControls;
use crate::app::media::playback::session::service::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct MediaSessionState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
}

#[derive(Default)]
pub struct MediaRuntimeState {
    pub stream: StreamRuntimeState,
    pub paused_seek_epoch: AtomicU32,
    pub preview_frame_epoch: AtomicU32,
}

pub struct MediaControlState {
    pub audio: Arc<AudioControls>,
    pub timing: Arc<TimingControls>,
    pub debug: Arc<DebugControls>,
}

impl Default for MediaControlState {
    fn default() -> Self {
        Self {
            audio: Arc::new(AudioControls::default()),
            timing: Arc::new(TimingControls::default()),
            debug: Arc::new(DebugControls::default()),
        }
    }
}

#[derive(Default)]
pub struct MediaCacheState {
    pub recorder: Mutex<Option<CacheRecorderSession>>,
}
