use super::{RendererInner, VideoFrame, FRAME_QUEUE_CAPACITY};

pub(super) fn pick_frame_for_present(
    inner: &RendererInner,
    now_media_seconds: f64,
) -> Option<VideoFrame> {
    let present_lead = 0.010;
    let deadline = now_media_seconds + present_lead;
    let mut queue = inner.queued_frames.lock().ok()?;
    let pts_seconds = queue.front()?.pts_seconds;
    if !pts_seconds.is_finite() || pts_seconds <= deadline {
        return queue.pop_front();
    }
    None
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
