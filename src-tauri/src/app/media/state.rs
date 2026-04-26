use crate::app::media::error::MediaError;
use crate::app::media::library::MediaLibraryService;
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::events::{
    MediaEventEnvelope, MEDIA_PLAYBACK_STATE_EVENT, MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::playback::session::service::MediaPlaybackService;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct StreamRuntimeState {
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    pending_seek_seconds: Mutex<Option<f64>>,
    latest_position_seconds: Mutex<f64>,
    generation: AtomicU32,
}

pub type DecodeStreamHandles = (Option<Arc<AtomicBool>>, Option<JoinHandle<()>>);

impl StreamRuntimeState {
    pub fn take_decode_stream_handles(
        &self,
    ) -> Result<DecodeStreamHandles, crate::app::media::error::MediaError> {
        let stop_flag = self
            .stop_flag
            .lock()
            .map_err(|_| crate::app::media::error::MediaError::state_poisoned_lock("stream state"))?
            .take();
        let thread = self
            .thread
            .lock()
            .map_err(|_| {
                crate::app::media::error::MediaError::state_poisoned_lock("stream thread")
            })?
            .take();
        Ok((stop_flag, thread))
    }

    pub fn install_decode_stream_handle(
        &self,
        stop_flag: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.stop_flag.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("stream state")
        })? = Some(stop_flag);
        *self.thread.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("stream thread")
        })? = Some(thread);
        Ok(())
    }

    pub fn request_stop(handles: &DecodeStreamHandles) {
        if let Some(flag) = handles.0.as_ref() {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn join(handles: DecodeStreamHandles) {
        if let Some(handle) = handles.1 {
            let _ = handle.join();
        }
    }

    pub fn set_latest_position_seconds(
        &self,
        position_seconds: f64,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.latest_position_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("latest position state")
        })? = position_seconds.max(0.0);
        Ok(())
    }

    pub fn latest_position_seconds(&self) -> Result<f64, crate::app::media::error::MediaError> {
        let value = *self.latest_position_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("latest position state")
        })?;
        Ok(value.max(0.0))
    }

    pub fn advance_generation(&self) -> u32 {
        self.generation
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    pub fn current_generation(&self) -> u32 {
        self.generation.load(Ordering::Relaxed)
    }

    pub fn is_generation_current(&self, generation: u32) -> bool {
        self.current_generation() == generation
    }

    pub fn set_pending_seek_seconds(
        &self,
        position_seconds: f64,
    ) -> Result<(), crate::app::media::error::MediaError> {
        *self.pending_seek_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
        })? = Some(position_seconds.max(0.0));
        Ok(())
    }

    pub fn reset_pending_seek_to_zero(&self) -> Result<(), crate::app::media::error::MediaError> {
        *self.pending_seek_seconds.lock().map_err(|_| {
            crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
        })? = Some(0.0);
        Ok(())
    }

    pub fn take_pending_seek_seconds(
        &self,
    ) -> Result<Option<f64>, crate::app::media::error::MediaError> {
        Ok(self
            .pending_seek_seconds
            .lock()
            .map_err(|_| {
                crate::app::media::error::MediaError::state_poisoned_lock("pending seek state")
            })?
            .take())
    }
}

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

#[derive(Default)]
pub struct AudioControls {
    volume_bits: AtomicU32,
    muted: AtomicBool,
}

#[derive(Default)]
pub struct TimingControls {
    playback_rate_bits: AtomicU32,
}

impl AudioControls {
    pub fn volume(&self) -> f32 {
        let bits = self.volume_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.min(1.0)
        } else {
            1.0
        }
    }

    pub fn set_volume(&self, value: f32) {
        let normalized = value.clamp(0.0, 1.0);
        self.volume_bits
            .store(normalized.to_bits(), Ordering::Relaxed);
    }

    pub fn muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn set_muted(&self, value: bool) {
        self.muted.store(value, Ordering::Relaxed);
    }
}

impl TimingControls {
    pub fn playback_rate(&self) -> f32 {
        let bits = self.playback_rate_bits.load(Ordering::Relaxed);
        let value = f32::from_bits(bits);
        if value.is_finite() && value > 0.0 {
            value.clamp(0.25, 3.0)
        } else {
            1.0
        }
    }

    pub fn set_playback_rate(&self, value: f32) {
        let normalized = value.clamp(0.25, 3.0);
        self.playback_rate_bits
            .store(normalized.to_bits(), Ordering::Relaxed);
    }
}

pub fn playback<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaPlaybackService>, String> {
    state
        .playback
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())
}

pub fn library<'a>(
    state: &'a State<'a, MediaState>,
) -> Result<MutexGuard<'a, MediaLibraryService>, String> {
    state
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())
}

pub fn emit_snapshot(
    app: &AppHandle,
    state: &State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    emit_snapshot_with_request_id(app, state, None)
}

pub fn emit_snapshot_with_request_id(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    request_id: Option<String>,
) -> Result<MediaSnapshot, String> {
    let snapshot = snapshot_from_state(state)?;
    let envelope = MediaEventEnvelope {
        protocol_version: MEDIA_PROTOCOL_VERSION,
        event_type: "playback_state",
        request_id: request_id.clone(),
        emitted_at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        payload: snapshot.clone(),
    };
    app.emit(MEDIA_PLAYBACK_STATE_EVENT, &envelope)
        .map_err(|err| format!("emit playback state failed: {err}"))?;
    Ok(snapshot)
}

pub fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .library
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("media library state").to_string())?
        .state();
    let playback = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.state()
    };
    Ok(MediaSnapshot { playback, library })
}
