use super::RendererInner;
use crate::app::media::playback::runtime::emit_debug;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub(super) const FRAME_QUEUE_TARGET_BASE_REALTIME: usize = 1;
pub(super) const FRAME_QUEUE_TARGET_BASE_NON_REALTIME: usize = 4;
const QUEUE_GROW_HOLD_AFTER_RESET_REALTIME: Duration = Duration::from_millis(3000);
const QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME: Duration = Duration::from_millis(350);

pub(super) fn base_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        FRAME_QUEUE_TARGET_BASE_NON_REALTIME
    }
}

pub(super) fn max_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        super::FRAME_QUEUE_HARD_CAPACITY
    }
}

pub(super) fn queue_growth_hold_after_reset(inner: &RendererInner) -> Duration {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        QUEUE_GROW_HOLD_AFTER_RESET_REALTIME
    } else {
        QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME
    }
}

pub(super) fn emit_queue_capacity_debug(
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
