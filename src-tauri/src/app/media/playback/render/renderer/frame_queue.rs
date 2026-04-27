use super::renderer_state::{RendererInner, FRAME_QUEUE_CAPACITY};
use super::VideoFrame;

pub(super) fn pick_frame_for_present(
    inner: &RendererInner,
    now_media_seconds: f64,
) -> Option<VideoFrame> {
    let present_lead = 0.004;
    let deadline = now_media_seconds + present_lead;
    let mut queue = inner.queued_frames.lock().ok()?;
    let mut selected = None;
    while let Some(frame) = queue.front() {
        if !frame.pts_seconds.is_finite() || frame.pts_seconds <= deadline {
            selected = queue.pop_front();
            continue;
        }
        break;
    }
    selected
}

pub(super) fn submit_frame_to_queue(inner: &RendererInner, frame: VideoFrame) {
    if let Ok(mut pts_guard) = inner.last_queued_pts.lock() {
        if let Some(last_pts) = *pts_guard {
            if frame.pts_seconds.is_finite() && frame.pts_seconds + 0.001 < last_pts {
                return;
            }
        }
        if frame.pts_seconds.is_finite() {
            *pts_guard = Some(frame.pts_seconds);
        }
    }
    if let Ok(mut queue) = inner.queued_frames.lock() {
        while queue.len() >= FRAME_QUEUE_CAPACITY {
            queue.pop_front();
        }
        queue.push_back(frame);
    }
    if let Ok(mut pending) = inner.pending_render.lock() {
        *pending = true;
        inner.render_cv.notify_one();
    }
}
