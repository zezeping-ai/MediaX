use super::renderer_state::{recycle_frame, RendererInner, FRAME_QUEUE_HARD_CAPACITY};
use super::QueuedFrame;

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
    let remaining_queue_depth = queue.len();
    drop(queue);
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
