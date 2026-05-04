use crate::app::media::playback::events::MediaAudioMeterPayload;
use crate::app::media::playback::runtime::emit::emit_audio_meter_payloads;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::AppHandle;

use super::spectrum::compute_spectrum;

const AUDIO_SPECTRUM_REFRESH_MS: u64 = 48;
const AUDIO_METER_WAIT_TIMEOUT_MS: u64 = 64;

#[derive(Default)]
pub(crate) struct SharedAudioMeterState {
    sequence: u64,
    snapshot: Option<AudioMeterSnapshot>,
}

pub(crate) struct SharedAudioMeterCell {
    state: Mutex<SharedAudioMeterState>,
    wake: Condvar,
}

#[derive(Clone)]
pub(crate) struct AudioMeterSnapshot {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) left_peak: f32,
    pub(crate) right_peak: f32,
    pub(crate) left_samples: Vec<f32>,
    pub(crate) right_samples: Vec<f32>,
}

pub(crate) type SharedAudioMeter = Arc<SharedAudioMeterCell>;

pub(crate) fn create_shared_audio_meter() -> SharedAudioMeter {
    Arc::new(SharedAudioMeterCell {
        state: Mutex::new(SharedAudioMeterState::default()),
        wake: Condvar::new(),
    })
}

pub(crate) fn publish_snapshot(shared: &SharedAudioMeter, snapshot: AudioMeterSnapshot) {
    if let Ok(mut state) = shared.state.lock() {
        state.sequence = state.sequence.saturating_add(1);
        state.snapshot = Some(snapshot);
        shared.wake.notify_one();
    }
}

pub(crate) fn notify_meter_shutdown(shared: &SharedAudioMeter) {
    shared.wake.notify_all();
}

pub(crate) fn spawn_audio_meter_emitter(
    app: AppHandle,
    shared: SharedAudioMeter,
    stop_flag: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut last_emitted_sequence = 0u64;
        let mut spectrum_cache = SpectrumCache::default();
        while !stop_flag.load(Ordering::Relaxed) {
            let next_payload = wait_for_snapshot(&shared, &stop_flag, last_emitted_sequence);
            if let Some((sequence, Some(snapshot))) = next_payload {
                last_emitted_sequence = sequence;
                emit_audio_meter_payloads(&app, build_payload(snapshot, &mut spectrum_cache));
            }
        }
    })
}

fn wait_for_snapshot(
    shared: &SharedAudioMeter,
    stop_flag: &Arc<AtomicBool>,
    last_emitted_sequence: u64,
) -> Option<(u64, Option<AudioMeterSnapshot>)> {
    let mut state = shared.state.lock().ok()?;
    loop {
        if state.sequence > last_emitted_sequence {
            return Some((state.sequence, state.snapshot.clone()));
        }
        if stop_flag.load(Ordering::Relaxed) {
            return None;
        }
        let (next_state, _) = shared
            .wake
            .wait_timeout(state, Duration::from_millis(AUDIO_METER_WAIT_TIMEOUT_MS))
            .ok()?;
        state = next_state;
    }
}

#[derive(Default)]
struct SpectrumCache {
    last_refresh_at: Option<std::time::Instant>,
    sample_rate: u32,
    left_spectrum: Vec<f32>,
    right_spectrum: Vec<f32>,
}

fn build_payload(
    snapshot: AudioMeterSnapshot,
    spectrum_cache: &mut SpectrumCache,
) -> MediaAudioMeterPayload {
    refresh_spectrum_cache_if_needed(spectrum_cache, &snapshot);
    MediaAudioMeterPayload {
        sample_rate: snapshot.sample_rate,
        channels: snapshot.channels,
        left_peak: snapshot.left_peak,
        right_peak: snapshot.right_peak,
        left_spectrum: spectrum_cache.left_spectrum.clone(),
        right_spectrum: spectrum_cache.right_spectrum.clone(),
    }
}

fn refresh_spectrum_cache_if_needed(
    spectrum_cache: &mut SpectrumCache,
    snapshot: &AudioMeterSnapshot,
) {
    let now = std::time::Instant::now();
    let should_refresh = spectrum_cache.sample_rate != snapshot.sample_rate
        || spectrum_cache.last_refresh_at.is_none()
        || spectrum_cache
            .last_refresh_at
            .is_some_and(|last_refresh_at| {
                now.duration_since(last_refresh_at)
                    >= Duration::from_millis(AUDIO_SPECTRUM_REFRESH_MS)
            });
    if !should_refresh {
        return;
    }

    spectrum_cache.sample_rate = snapshot.sample_rate;
    spectrum_cache.left_spectrum = compute_spectrum(&snapshot.left_samples, snapshot.sample_rate);
    spectrum_cache.right_spectrum = compute_spectrum(&snapshot.right_samples, snapshot.sample_rate);
    spectrum_cache.last_refresh_at = Some(now);
}
