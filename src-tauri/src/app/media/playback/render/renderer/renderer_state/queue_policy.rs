use super::{QueueTuningState, RendererInner};
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::app::media::playback::runtime::emit_debug;

const QUEUE_GROW_LAG_MS: f64 = 24.0;
const QUEUE_SHRINK_LAG_MS: f64 = 8.0;
const QUEUE_GROW_STREAK: u8 = 2;
const QUEUE_SHRINK_STREAK: u8 = 18;

pub(super) fn update_dynamic_queue_capacity(
    inner: &RendererInner,
    lag_ms: f64,
    has_presented_frame: bool,
    remaining_queue_depth: usize,
) {
    let Ok(mut tuning) = inner.queue_tuning.lock() else {
        return;
    };
    let min_capacity = base_target_queue_capacity(inner);
    let max_capacity = max_target_queue_capacity(inner);
    let current_capacity = inner
        .target_queue_capacity
        .load(Ordering::Relaxed)
        .clamp(min_capacity, max_capacity);
    if max_capacity <= min_capacity {
        if current_capacity != min_capacity {
            inner
                .target_queue_capacity
                .store(min_capacity, Ordering::Relaxed);
        }
        *tuning = QueueTuningState::default();
        return;
    }
    let queue_growth_held = inner
        .queue_growth_hold_until
        .lock()
        .ok()
        .and_then(|value| *value)
        .map(|deadline| Instant::now() < deadline)
        .unwrap_or(false);
    if has_presented_frame && (lag_ms >= QUEUE_GROW_LAG_MS || (lag_ms >= 12.0 && remaining_queue_depth == 0)) {
        if queue_growth_held {
            tuning.late_present_streak = 0;
            tuning.healthy_present_streak = 0;
            return;
        }
        tuning.late_present_streak = tuning.late_present_streak.saturating_add(1);
        tuning.healthy_present_streak = 0;
        if tuning.late_present_streak >= QUEUE_GROW_STREAK && current_capacity < max_capacity {
            let next_capacity = current_capacity + 1;
            inner
                .target_queue_capacity
                .store(next_capacity, Ordering::Relaxed);
            emit_queue_capacity_debug(
                inner,
                "renderer_queue_grow",
                current_capacity,
                next_capacity,
                lag_ms,
                remaining_queue_depth,
            );
            tuning.late_present_streak = 0;
        }
        return;
    }
    if has_presented_frame && lag_ms <= QUEUE_SHRINK_LAG_MS && remaining_queue_depth >= 1 {
        tuning.healthy_present_streak = tuning.healthy_present_streak.saturating_add(1);
        tuning.late_present_streak = 0;
        if tuning.healthy_present_streak >= QUEUE_SHRINK_STREAK && current_capacity > min_capacity {
            let next_capacity = current_capacity - 1;
            inner
                .target_queue_capacity
                .store(next_capacity, Ordering::Relaxed);
            emit_queue_capacity_debug(
                inner,
                "renderer_queue_shrink",
                current_capacity,
                next_capacity,
                lag_ms,
                remaining_queue_depth,
            );
            tuning.healthy_present_streak = 0;
        }
        return;
    }
    tuning.late_present_streak = 0;
    tuning.healthy_present_streak = 0;
}

pub(super) fn base_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        super::FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        super::FRAME_QUEUE_TARGET_BASE_NON_REALTIME
    }
}

pub(super) fn max_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        super::FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        super::FRAME_QUEUE_HARD_CAPACITY
    }
}

pub(super) fn queue_growth_hold_after_reset(inner: &RendererInner) -> std::time::Duration {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        super::QUEUE_GROW_HOLD_AFTER_RESET_REALTIME
    } else {
        super::QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME
    }
}

fn emit_queue_capacity_debug(
    inner: &RendererInner,
    stage: &'static str,
    from_capacity: usize,
    to_capacity: usize,
    lag_ms: f64,
    remaining_queue_depth: usize,
) {
    let Some(app_handle) = inner
        .app_handle
        .lock()
        .ok()
        .and_then(|value| value.clone())
    else {
        return;
    };
    emit_debug(
        &app_handle,
        stage,
        format!(
            "capacity {}->{} lag_ms={:.3} remaining_queue_depth={} realtime={}",
            from_capacity,
            to_capacity,
            lag_ms,
            remaining_queue_depth,
            inner.is_realtime_source.load(Ordering::Relaxed),
        ),
    );
}

