use crate::app::media::playback::events::MediaAudioMeterPayload;
use crate::app::media::playback::runtime::emit::emit_audio_meter_payloads;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use super::spectrum::compute_spectrum;

const AUDIO_METER_POLL_MS: u64 = 16;

#[derive(Default)]
pub(crate) struct SharedAudioMeterState {
    sequence: u64,
    snapshot: Option<AudioMeterSnapshot>,
}

#[derive(Clone)]
pub(super) struct AudioMeterSnapshot {
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) left_peak: f32,
    pub(super) right_peak: f32,
    pub(super) left_samples: Vec<f32>,
    pub(super) right_samples: Vec<f32>,
}

pub(crate) type SharedAudioMeter = Arc<Mutex<SharedAudioMeterState>>;

pub(crate) fn create_shared_audio_meter() -> SharedAudioMeter {
    Arc::new(Mutex::new(SharedAudioMeterState::default()))
}

pub(crate) fn publish_snapshot(shared: &SharedAudioMeter, snapshot: AudioMeterSnapshot) {
    if let Ok(mut state) = shared.lock() {
        state.sequence = state.sequence.saturating_add(1);
        state.snapshot = Some(snapshot);
    }
}

pub(crate) fn spawn_audio_meter_emitter(
    app: AppHandle,
    shared: SharedAudioMeter,
    stop_flag: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last_emitted_sequence = 0u64;
        while !stop_flag.load(Ordering::Relaxed) {
            if !app
                .state::<crate::app::media::state::MediaState>()
                .controls
                .debug
                .frontend_diagnostics_enabled()
            {
                thread::sleep(Duration::from_millis(AUDIO_METER_POLL_MS));
                continue;
            }
            let next_payload = shared.lock().ok().and_then(|state| {
                if state.sequence > last_emitted_sequence {
                    Some((state.sequence, state.snapshot.clone()))
                } else {
                    None
                }
            });
            if let Some((sequence, Some(snapshot))) = next_payload {
                last_emitted_sequence = sequence;
                emit_audio_meter_payloads(&app, build_payload(snapshot));
            }
            thread::sleep(Duration::from_millis(AUDIO_METER_POLL_MS));
        }
    })
}

fn build_payload(snapshot: AudioMeterSnapshot) -> MediaAudioMeterPayload {
    MediaAudioMeterPayload {
        sample_rate: snapshot.sample_rate,
        channels: snapshot.channels,
        left_peak: snapshot.left_peak,
        right_peak: snapshot.right_peak,
        left_spectrum: compute_spectrum(&snapshot.left_samples, snapshot.sample_rate),
        right_spectrum: compute_spectrum(&snapshot.right_samples, snapshot.sample_rate),
    }
}
