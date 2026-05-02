use super::renderer_state::{recycle_frame, RendererInner, FRAME_QUEUE_HARD_CAPACITY};
use super::QueuedFrame;
use crate::app::media::playback::runtime::emit_debug;
use std::time::{Duration, Instant};

pub(super) struct FramePresentSelection {
    pub frame: Option<QueuedFrame>,
    pub remaining_queue_depth: usize,
}

pub(super) fn pick_frame_for_present(
    inner: &RendererInner,
    now_media_seconds: f64,
) -> FramePresentSelection {
    let present_lead = 0.004;
    let stale_drop_lag = 0.050;
    let deadline = now_media_seconds + present_lead;
    let stale_deadline = now_media_seconds - stale_drop_lag;
    let Ok(mut queue) = inner.queued_frames.lock() else {
        return FramePresentSelection {
            frame: None,
            remaining_queue_depth: 0,
        };
    };
    let queue_before: Vec<f64> = queue
        .iter()
        .take(6)
        .map(QueuedFrame::pts_seconds)
        .collect();
    let queue_depth_before = queue.len();
    let mut dropped_stale = false;
    let mut dropped_stale_count = 0usize;
    while let Some(frame) = queue.front() {
        let pts_seconds = frame.pts_seconds();
        if !pts_seconds.is_finite() || pts_seconds < stale_deadline {
            dropped_stale = true;
            dropped_stale_count = dropped_stale_count.saturating_add(1);
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
    let selected_pts = selected.as_ref().map(QueuedFrame::pts_seconds);
    let queue_after: Vec<f64> = queue
        .iter()
        .take(6)
        .map(QueuedFrame::pts_seconds)
        .collect();
    let remaining_queue_depth = queue.len();
    drop(queue);
    maybe_emit_frame_queue_trace(
        inner,
        now_media_seconds,
        deadline,
        stale_deadline,
        queue_depth_before,
        &queue_before,
        dropped_stale_count,
        selected_pts,
        &queue_after,
        remaining_queue_depth,
    );
    if dropped_stale || selected.is_some() {
        inner.frame_slot_cv.notify_all();
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

fn maybe_emit_frame_queue_trace(
    inner: &RendererInner,
    now_media_seconds: f64,
    deadline: f64,
    stale_deadline: f64,
    queue_depth_before: usize,
    queue_before: &[f64],
    dropped_stale_count: usize,
    selected_pts: Option<f64>,
    queue_after: &[f64],
    remaining_queue_depth: usize,
) {
    let should_trace = dropped_stale_count > 0
        || (queue_depth_before >= 4 && selected_pts.is_some() && remaining_queue_depth >= 3);
    if !should_trace {
        return;
    }
    let Ok(mut last_trace_at) = inner.last_frame_queue_trace_at.lock() else {
        return;
    };
    let now = Instant::now();
    if last_trace_at
        .as_ref()
        .is_some_and(|last| now.saturating_duration_since(*last) < Duration::from_millis(700))
    {
        return;
    }
    *last_trace_at = Some(now);
    drop(last_trace_at);
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
        "frame_queue_pick",
        format!(
            "now={now_media_seconds:.3}s deadline={deadline:.3}s stale_deadline={stale_deadline:.3}s before_depth={queue_depth_before} remaining_depth={remaining_queue_depth} dropped_stale={dropped_stale_count} selected={} before=[{}] after=[{}]",
            selected_pts
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "none".to_string()),
            format_pts_list(queue_before),
            format_pts_list(queue_after),
        ),
    );
}

fn format_pts_list(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format!("{value:.3}"))
        .collect::<Vec<_>>()
        .join(",")
}
