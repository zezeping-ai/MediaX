use super::renderer_state::{recycle_frame, source_frame_duration_seconds, RendererInner, FRAME_QUEUE_HARD_CAPACITY};
use crate::app::media::playback::sync::PresentationPolicy;
use super::QueuedFrame;

pub(super) struct FramePresentSelection {
    pub frame: Option<QueuedFrame>,
    pub remaining_queue_depth: usize,
}

pub(super) fn pick_frame_for_present(
    inner: &RendererInner,
    master_seconds: f64,
) -> FramePresentSelection {
    let policy = PresentationPolicy::from_frame_duration(source_frame_duration_seconds(inner));
    let deadline = policy.presentation_deadline(master_seconds);
    let stale_deadline = policy.stale_threshold(master_seconds);
    let Ok(mut queue) = inner.queued_frames.lock() else {
        return FramePresentSelection {
            frame: None,
            remaining_queue_depth: 0,
        };
    };
    while let Some(frame) = queue.front() {
        let pts_seconds = frame.pts_seconds();
        if !pts_seconds.is_finite() || pts_seconds < stale_deadline {
            let dropped = queue.pop_front().expect("frame exists");
            recycle_frame(inner, dropped);
            continue;
        }
        break;
    }
    // When decode runs ahead, several frames can be due at once. Present the newest due frame.
    let mut selected = None;
    while queue
        .front()
        .map(|frame| frame.pts_seconds())
        .is_some_and(|pts_seconds| pts_seconds.is_finite() && pts_seconds <= deadline)
    {
        if let Some(skipped) = selected.take() {
            recycle_frame(inner, skipped);
        }
        selected = queue.pop_front();
    }
    let remaining_queue_depth = queue.len();
    drop(queue);
    if selected.is_some() {
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
