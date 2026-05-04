use super::backpressure_profile::{AudioBackpressureProfile, BackpressureClass, SourceKind};
use crate::app::media::playback::runtime::emit::emit_debug_throttled;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::AppHandle;

const AUDIO_BACKPRESSURE_CADENCE_WINDOW_MS: u64 = 1200;
const AUDIO_BACKPRESSURE_CADENCE_FREQUENT_THRESHOLD: u32 = 3;
const AUDIO_BACKPRESSURE_SUMMARY_WINDOW_MS: u64 = 10_000;
const AUDIO_BACKPRESSURE_SUMMARY_SAMPLE_EVERY: u32 = 16;
const AUDIO_HIGH_WATER_DEFER_MIN_MS: u64 = 2;
const AUDIO_HIGH_WATER_DEFER_MAX_MS: u64 = 6;
const AUDIO_HIGH_WATER_DEFER_GUARD_MAX_MS: u64 = 3;
const AUDIO_UNDERRUN_GUARD_WINDOW_MS: u64 = 1500;

static AUDIO_BACKPRESSURE_CADENCE: OnceLock<Mutex<BackpressureCadence>> = OnceLock::new();
static AUDIO_BACKPRESSURE_SUMMARY: OnceLock<Mutex<BackpressureSummary>> = OnceLock::new();
static AUDIO_HIGH_WATER_DEFER_UNTIL: OnceLock<Mutex<Option<std::time::Instant>>> = OnceLock::new();
static AUDIO_UNDERRUN_GUARD: OnceLock<Mutex<UnderrunGuardState>> = OnceLock::new();

#[derive(Default)]
struct BackpressureCadence {
    window_started_at: Option<std::time::Instant>,
    event_count: u32,
}

#[derive(Default)]
struct BackpressureSummary {
    window_started_at: Option<std::time::Instant>,
    seen_total: u32,
    audio_only_count: u32,
    av_file_count: u32,
    av_realtime_count: u32,
    av_network_count: u32,
    high_count: u32,
    normal_count: u32,
    low_count: u32,
    queued_seconds_sum: f64,
    queued_seconds_samples: u32,
}

#[derive(Default)]
struct UnderrunGuardState {
    last_underrun_count: u64,
    guard_until: Option<std::time::Instant>,
}

mod state_windows {
    use super::*;

    pub(super) fn with_cadence_mut<R>(f: impl FnOnce(&mut BackpressureCadence) -> R) -> Option<R> {
        let cadence = AUDIO_BACKPRESSURE_CADENCE.get_or_init(|| Mutex::new(BackpressureCadence::default()));
        let Ok(mut cadence) = cadence.lock() else {
            return None;
        };
        Some(f(&mut cadence))
    }

    pub(super) fn with_summary_mut<R>(f: impl FnOnce(&mut BackpressureSummary) -> R) -> Option<R> {
        let summary = AUDIO_BACKPRESSURE_SUMMARY.get_or_init(|| Mutex::new(BackpressureSummary::default()));
        let Ok(mut summary) = summary.lock() else {
            return None;
        };
        Some(f(&mut summary))
    }

    pub(super) fn with_defer_until_mut<R>(f: impl FnOnce(&mut Option<std::time::Instant>) -> R) -> Option<R> {
        let defer_until = AUDIO_HIGH_WATER_DEFER_UNTIL.get_or_init(|| Mutex::new(None));
        let Ok(mut defer_until) = defer_until.lock() else {
            return None;
        };
        Some(f(&mut defer_until))
    }

    pub(super) fn with_underrun_guard_mut<R>(f: impl FnOnce(&mut UnderrunGuardState) -> R) -> Option<R> {
        let guard = AUDIO_UNDERRUN_GUARD.get_or_init(|| Mutex::new(UnderrunGuardState::default()));
        let Ok(mut guard) = guard.lock() else {
            return None;
        };
        Some(f(&mut guard))
    }
}

pub(super) fn record_audio_backpressure_and_check_frequent() -> bool {
    state_windows::with_cadence_mut(|cadence| {
        let now = std::time::Instant::now();
        let window_elapsed_ms = cadence
            .window_started_at
            .map(|started| now.saturating_duration_since(started).as_millis() as u64)
            .unwrap_or(u64::MAX);
        if window_elapsed_ms > AUDIO_BACKPRESSURE_CADENCE_WINDOW_MS {
            cadence.window_started_at = Some(now);
            cadence.event_count = 1;
            return false;
        }
        cadence.event_count = cadence.event_count.saturating_add(1);
        cadence.event_count >= AUDIO_BACKPRESSURE_CADENCE_FREQUENT_THRESHOLD
    })
    .unwrap_or(false)
}

pub(super) fn record_audio_backpressure_summary(app: &AppHandle, profile: &AudioBackpressureProfile) {
    let _ = state_windows::with_summary_mut(|summary| {
        let now = std::time::Instant::now();
        let window_elapsed_ms = summary
            .window_started_at
            .map(|started| now.saturating_duration_since(started).as_millis() as u64)
            .unwrap_or(u64::MAX);
        if window_elapsed_ms > AUDIO_BACKPRESSURE_SUMMARY_WINDOW_MS {
            emit_audio_backpressure_summary(app, summary);
            *summary = BackpressureSummary::default();
            summary.window_started_at = Some(now);
        } else if summary.window_started_at.is_none() {
            summary.window_started_at = Some(now);
        }
        summary.seen_total = summary.seen_total.saturating_add(1);
        if summary.seen_total % AUDIO_BACKPRESSURE_SUMMARY_SAMPLE_EVERY != 0 {
            return;
        }
        accumulate_source_kind(summary, profile.source_kind);
        accumulate_backpressure_class(summary, profile.class);
        summary.queued_seconds_sum += profile.queued_seconds;
        summary.queued_seconds_samples = summary.queued_seconds_samples.saturating_add(1);
    });
}

pub(super) fn should_defer_audio_packet_for_high_water(
    profile: &AudioBackpressureProfile,
) -> bool {
    if !profile.high_water {
        return false;
    }
    state_windows::with_defer_until_mut(|defer_until| {
        defer_until
            .as_ref()
            .map(|deadline| std::time::Instant::now() < *deadline)
            .unwrap_or(false)
    })
    .unwrap_or(false)
}

pub(super) fn mark_audio_high_water_defer_window(defer_ms: u64) {
    let _ = state_windows::with_defer_until_mut(|defer_until| {
        *defer_until = Some(std::time::Instant::now() + Duration::from_millis(defer_ms));
    });
}

pub(super) fn defer_ms_for_high_water(profile: &AudioBackpressureProfile) -> u64 {
    let overshoot_seconds = (profile.queued_seconds - profile.refill_floor_seconds).max(0.0);
    let scaled = (overshoot_seconds / 0.08).floor() as u64;
    let defer_max_ms = dynamic_high_water_defer_max_ms(profile.underrun_count);
    (AUDIO_HIGH_WATER_DEFER_MIN_MS + scaled).clamp(AUDIO_HIGH_WATER_DEFER_MIN_MS, defer_max_ms)
}

fn accumulate_source_kind(summary: &mut BackpressureSummary, source_kind: SourceKind) {
    match source_kind {
        SourceKind::AudioOnly => summary.audio_only_count = summary.audio_only_count.saturating_add(1),
        SourceKind::AvRealtime => summary.av_realtime_count = summary.av_realtime_count.saturating_add(1),
        SourceKind::AvNetwork => summary.av_network_count = summary.av_network_count.saturating_add(1),
        SourceKind::AvFile => summary.av_file_count = summary.av_file_count.saturating_add(1),
    }
}

fn accumulate_backpressure_class(summary: &mut BackpressureSummary, class: BackpressureClass) {
    match class {
        BackpressureClass::HighWater => summary.high_count = summary.high_count.saturating_add(1),
        BackpressureClass::NormalWater => summary.normal_count = summary.normal_count.saturating_add(1),
        BackpressureClass::LowWater => summary.low_count = summary.low_count.saturating_add(1),
    }
}

fn emit_audio_backpressure_summary(app: &AppHandle, summary: &BackpressureSummary) {
    let total = summary
        .high_count
        .saturating_add(summary.normal_count)
        .saturating_add(summary.low_count);
    if summary.seen_total == 0 || total == 0 || summary.queued_seconds_samples == 0 {
        return;
    }
    let avg_queued_seconds = summary.queued_seconds_sum / (summary.queued_seconds_samples as f64);
    let total_f64 = total as f64;
    let high_pct = (summary.high_count as f64 / total_f64) * 100.0;
    let normal_pct = (summary.normal_count as f64 / total_f64) * 100.0;
    let low_pct = (summary.low_count as f64 / total_f64) * 100.0;
    let decision_hint = if high_pct >= 75.0 {
        "queue_saturated"
    } else if low_pct >= 35.0 {
        "supply_limited"
    } else {
        "mixed"
    };
    let source_kind = dominant_source_kind(summary);
    emit_debug_throttled(
        app,
        "audio_decoder_backpressure_summary",
        format!(
            "window={}ms sampled={} seen={} sample_every={} high={}({:.1}%) normal={}({:.1}%) low={}({:.1}%) avg_queued={:.3}s source_kind={} decision_hint={}",
            AUDIO_BACKPRESSURE_SUMMARY_WINDOW_MS,
            total,
            summary.seen_total,
            AUDIO_BACKPRESSURE_SUMMARY_SAMPLE_EVERY,
            summary.high_count,
            high_pct,
            summary.normal_count,
            normal_pct,
            summary.low_count,
            low_pct,
            avg_queued_seconds,
            source_kind,
            decision_hint
        ),
        0,
    );
}

fn dominant_source_kind(summary: &BackpressureSummary) -> &'static str {
    let mut best_kind = "mixed";
    let mut best_count = 0u32;
    for (kind, count) in [
        ("audio_only", summary.audio_only_count),
        ("av_file", summary.av_file_count),
        ("av_realtime", summary.av_realtime_count),
        ("av_network", summary.av_network_count),
    ] {
        if count > best_count {
            best_kind = kind;
            best_count = count;
        } else if count == best_count && count != 0 {
            best_kind = "mixed";
        }
    }
    best_kind
}

fn dynamic_high_water_defer_max_ms(underrun_count: u64) -> u64 {
    state_windows::with_underrun_guard_mut(|guard| {
        let now = std::time::Instant::now();
        if underrun_count > guard.last_underrun_count {
            guard.last_underrun_count = underrun_count;
            guard.guard_until = Some(now + Duration::from_millis(AUDIO_UNDERRUN_GUARD_WINDOW_MS));
        }
        if guard
            .guard_until
            .as_ref()
            .map(|deadline| now < *deadline)
            .unwrap_or(false)
        {
            return AUDIO_HIGH_WATER_DEFER_GUARD_MAX_MS;
        }
        AUDIO_HIGH_WATER_DEFER_MAX_MS
    })
    .unwrap_or(AUDIO_HIGH_WATER_DEFER_MAX_MS)
}
