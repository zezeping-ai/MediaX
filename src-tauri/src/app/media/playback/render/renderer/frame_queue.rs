use super::renderer_state::{recycle_frame, RendererInner, FRAME_QUEUE_HARD_CAPACITY};
use super::QueuedFrame;
use std::collections::VecDeque;

use crate::app::media::playback::runtime::emit_debug;

const PRESENT_LEAD_MIN_SECONDS: f64 = 0.004;
const PRESENT_LEAD_MAX_SECONDS: f64 = 0.018;
const PRESENT_LEAD_FRACTION_OF_FRAME: f64 = 0.50;
const STALE_DROP_LAG_MIN_SECONDS: f64 = 0.012;
const STALE_DROP_LAG_MAX_SECONDS: f64 = 0.050;
const STALE_DROP_LAG_FRACTION_OF_FRAME: f64 = 1.25;

pub(super) struct FramePresentSelection {
    pub frame: Option<QueuedFrame>,
    pub remaining_queue_depth: usize,
}

pub(super) fn present_lead_seconds_for_queue(queue: &VecDeque<QueuedFrame>) -> f64 {
    frame_interval_seconds_for_queue(queue)
        .map(|frame_interval| {
            (frame_interval * PRESENT_LEAD_FRACTION_OF_FRAME)
                .clamp(PRESENT_LEAD_MIN_SECONDS, PRESENT_LEAD_MAX_SECONDS)
        })
        .unwrap_or(PRESENT_LEAD_MIN_SECONDS)
}

fn stale_drop_lag_seconds_for_queue(queue: &VecDeque<QueuedFrame>) -> f64 {
    frame_interval_seconds_for_queue(queue)
        .map(|frame_interval| {
            (frame_interval * STALE_DROP_LAG_FRACTION_OF_FRAME)
                .clamp(STALE_DROP_LAG_MIN_SECONDS, STALE_DROP_LAG_MAX_SECONDS)
        })
        .unwrap_or(STALE_DROP_LAG_MAX_SECONDS)
}

fn frame_interval_seconds_for_queue(queue: &VecDeque<QueuedFrame>) -> Option<f64> {
    let mut frames = queue.iter();
    let head_pts = frames.next()?.pts_seconds();
    let next_pts = frames.next()?.pts_seconds();
    let frame_interval = (next_pts - head_pts).abs();
    frame_interval.is_finite().then_some(frame_interval)
}

fn should_preserve_single_poster_frame(
    queue: &VecDeque<QueuedFrame>,
    now_media_seconds: f64,
) -> bool {
    if queue.len() != 1 {
        return false;
    }
    let Some(frame) = queue.front() else {
        return false;
    };
    let pts_seconds = frame.pts_seconds();
    // Audio cover/poster frames are commonly injected with pts=0. After resume/seek to a later
    // position they look stale by timeline math, but dropping them leaves audio-only playback
    // without any visual frame.
    pts_seconds.is_finite() && pts_seconds <= 0.001 && now_media_seconds >= 0.5
}

pub(super) fn pick_frame_for_present(
    inner: &RendererInner,
    now_media_seconds: f64,
) -> FramePresentSelection {
    // Allow a modest early-present window so small main-thread / vsync jitter does not
    // force us to miss the frame and show it one wakeup late.
    let present_lead = inner
        .queued_frames
        .lock()
        .ok()
        .map(|queue| present_lead_seconds_for_queue(&queue))
        .unwrap_or(PRESENT_LEAD_MIN_SECONDS);
    let Ok(mut queue) = inner.queued_frames.lock() else {
        return FramePresentSelection {
            frame: None,
            remaining_queue_depth: 0,
        };
    };
    let stale_drop_lag = stale_drop_lag_seconds_for_queue(&queue);
    let deadline = now_media_seconds + present_lead;
    let stale_deadline = now_media_seconds - stale_drop_lag;
    let mut dropped_stale = false;
    let mut dropped_stale_count = 0usize;
    let mut first_dropped_pts = None;
    while let Some(frame) = queue.front() {
        if should_preserve_single_poster_frame(&queue, now_media_seconds) {
            break;
        }
        let pts_seconds = frame.pts_seconds();
        if !pts_seconds.is_finite() || pts_seconds < stale_deadline {
            dropped_stale = true;
            dropped_stale_count = dropped_stale_count.saturating_add(1);
            first_dropped_pts.get_or_insert(pts_seconds);
            let dropped = queue.pop_front().expect("frame exists");
            recycle_frame(inner, dropped);
            continue;
        }
        break;
    }
    let should_present_front = queue
        .front()
        .map(|frame| frame.pts_seconds())
        .is_some_and(|pts_seconds| pts_seconds.is_finite() && pts_seconds <= deadline);
    let selected = should_present_front.then(|| queue.pop_front().expect("frame exists"));
    let remaining_queue_depth = queue.len();
    let next_head_pts = queue.front().map(QueuedFrame::pts_seconds);
    drop(queue);
    if dropped_stale || selected.is_some() {
        inner.frame_slot_cv.notify_all();
    }
    if dropped_stale {
        if let Some(app_handle) = inner.app_handle.lock().ok().and_then(|value| value.clone()) {
            emit_debug(
                &app_handle,
                "renderer_stale_drop",
                format!(
                    "count={} first_pts={:.3?} now_media_s={:.3} stale_deadline_s={:.3} selected_pts={:.3?} next_head_pts={:.3?} remaining_queue_depth={}",
                    dropped_stale_count,
                    first_dropped_pts,
                    now_media_seconds,
                    stale_deadline,
                    selected.as_ref().map(QueuedFrame::pts_seconds),
                    next_head_pts,
                    remaining_queue_depth,
                ),
            );
        }
    }
    FramePresentSelection {
        frame: selected,
        remaining_queue_depth,
    }
}

pub(super) fn submit_frame_to_queue(inner: &RendererInner, frame: QueuedFrame) {
    let pts_seconds = frame.pts_seconds();
    if let Ok(mut pts_guard) = inner.last_queued_pts.lock() {
        if let Some(last_pts) = *pts_guard {
            if pts_seconds.is_finite() && pts_seconds + 0.001 < last_pts {
                recycle_frame(inner, frame);
                return;
            }
        }
        if pts_seconds.is_finite() {
            *pts_guard = Some(pts_seconds);
        }
    }
    if let Ok(mut queue) = inner.queued_frames.lock() {
        while queue.len() >= FRAME_QUEUE_HARD_CAPACITY {
            if let Some(dropped) = queue.pop_front() {
                recycle_frame(inner, dropped);
            }
        }
        queue.push_back(frame);
    }
    if let Ok(mut pending) = inner.pending_render.lock() {
        *pending = true;
        inner.render_cv.notify_one();
    }
}
