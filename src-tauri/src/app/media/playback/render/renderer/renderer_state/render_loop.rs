use super::clock::current_clock_seconds;
use super::queue_policy::update_dynamic_queue_capacity;
use super::{QueuedFrame, RendererInner, VideoScaleMode};
use crate::app::media::playback::render::renderer::frame_queue::{pick_frame_for_present, present_lead_seconds_for_queue};
use crate::app::media::playback::render::renderer::helpers::wait_for_render_signal;
use crate::app::media::playback::render::renderer::renderer_types::RenderStageTimings;
use crate::app::media::playback::runtime::emit_debug;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const RENDER_LOOP_ACTIVE_MAX_WAIT: Duration = Duration::from_millis(24);
const RENDER_LOOP_ACTIVE_MIN_WAIT: Duration = Duration::from_millis(1);
const RENDER_LOOP_IDLE_TICK: Duration = Duration::from_millis(120);
const RENDER_SLOW_PATH_TOTAL_LOG_THRESHOLD: Duration = Duration::from_millis(6);
const RENDER_SLOW_PATH_UPLOAD_LOG_THRESHOLD: Duration = Duration::from_millis(4);
const RENDER_SLOW_PATH_LAG_LOG_THRESHOLD_MS: f64 = 3.0;

pub(super) fn spawn_render_loop_thread(app_handle: tauri::AppHandle, inner: Arc<RendererInner>) {
    thread::spawn(move || {
        // Present continuously to align cadence with display vsync. When no new video
        // frame is due, sleep until the next frame deadline instead of waking at a fixed
        // high frequency; when idle we fall back to a much lower tick for resize upkeep.
        while !inner.stop.load(Ordering::Relaxed) {
            let wait_timeout = compute_render_wait_timeout(&inner);
            let timed_out = wait_for_render_signal(&inner, wait_timeout);
            inner.render_loop_wakeups.fetch_add(1, Ordering::Relaxed);
            if timed_out && !should_attempt_present_on_timeout(&inner) {
                continue;
            }
            if inner.render_task_in_flight.swap(true, Ordering::AcqRel) {
                thread::sleep(RENDER_LOOP_ACTIVE_MIN_WAIT);
                continue;
            }
            let _ = app_handle.run_on_main_thread({
                let inner = inner.clone();
                move || {
                    let now_media_seconds = current_clock_seconds(&inner);
                    let selection = pick_frame_for_present(&inner, now_media_seconds);
                    let frame = selection.frame;
                    if frame.is_none() && !timed_out {
                        inner.render_task_in_flight.store(false, Ordering::Release);
                        return;
                    }
                    inner.render_attempts.fetch_add(1, Ordering::Relaxed);
                    if frame.is_some() {
                        inner.render_uploads.fetch_add(1, Ordering::Relaxed);
                    }
                    if let Ok(mut guard) = inner.renderer.lock() {
                        if let Some(renderer) = guard.as_mut() {
                            let render_start = Instant::now();
                            renderer.set_video_scale_mode(VideoScaleMode::from_u8(
                                inner.video_scale_mode.load(Ordering::Relaxed),
                            ));
                            let render_stage_timings = renderer.render(frame.as_ref(), true);
                            inner
                                .current_surface_width
                                .store(renderer.config.width, Ordering::Relaxed);
                            inner
                                .current_surface_height
                                .store(renderer.config.height, Ordering::Relaxed);
                            inner.render_presents.fetch_add(1, Ordering::Relaxed);
                            let elapsed = render_start.elapsed();
                            inner.last_render_cost_micros.store(
                                u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
                                Ordering::Relaxed,
                            );
                            let lag_ms = frame
                                .as_ref()
                                .map(|f| ((now_media_seconds - f.pts_seconds()).max(0.0) * 1000.0) as f32)
                                .unwrap_or(0.0);
                            if let Ok(stage_timings) = render_stage_timings {
                                maybe_emit_slow_render_path_debug(
                                    &inner,
                                    frame.as_ref(),
                                    stage_timings,
                                    f64::from(lag_ms),
                                );
                            }
                            inner
                                .last_present_lag_ms_bits
                                .store(lag_ms.to_bits(), Ordering::Relaxed);
                            let presented_pts = frame
                                .as_ref()
                                .map(QueuedFrame::pts_seconds)
                                .filter(|value| value.is_finite());
                            inner.last_presented_pts_bits.store(
                                presented_pts.unwrap_or(f64::NAN).to_bits(),
                                Ordering::Relaxed,
                            );
                            update_dynamic_queue_capacity(
                                &inner,
                                f64::from(lag_ms),
                                presented_pts.is_some(),
                                selection.remaining_queue_depth,
                            );
                        }
                    }
                    inner.render_task_in_flight.store(false, Ordering::Release);
                }
            });
        }
    });
}

fn compute_render_wait_timeout(inner: &RendererInner) -> Duration {
    let now_media_seconds = current_clock_seconds(inner);
    let Some((next_pts_seconds, present_lead_seconds)) = inner
        .queued_frames
        .lock()
        .ok()
        .and_then(|queue| {
            queue
                .front()
                .map(QueuedFrame::pts_seconds)
                .filter(|value| value.is_finite())
                .map(|next_pts| (next_pts, present_lead_seconds_for_queue(&queue)))
        })
    else {
        return RENDER_LOOP_IDLE_TICK;
    };
    let seconds_until_due = (next_pts_seconds - (now_media_seconds + present_lead_seconds)).max(0.0);
    if seconds_until_due <= 0.0 {
        return Duration::ZERO;
    }
    Duration::from_secs_f64(seconds_until_due)
        .clamp(RENDER_LOOP_ACTIVE_MIN_WAIT, RENDER_LOOP_ACTIVE_MAX_WAIT)
}

fn should_attempt_present_on_timeout(inner: &RendererInner) -> bool {
    let now_media_seconds = current_clock_seconds(inner);
    inner
        .queued_frames
        .lock()
        .ok()
        .and_then(|queue| {
            queue
                .front()
                .map(QueuedFrame::pts_seconds)
                .filter(|value| value.is_finite())
                .map(|pts_seconds| (pts_seconds, present_lead_seconds_for_queue(&queue)))
        })
        .is_some_and(|(pts_seconds, present_lead_seconds)| {
            pts_seconds <= now_media_seconds + present_lead_seconds
        })
}

fn maybe_emit_slow_render_path_debug(
    inner: &RendererInner,
    frame: Option<&QueuedFrame>,
    stage_timings: RenderStageTimings,
    lag_ms: f64,
) {
    let total = stage_timings.total();
    let slowest_stage = [
        stage_timings.upload_frame,
        stage_timings.acquire_surface,
        stage_timings.encode_and_submit,
        stage_timings.present,
    ]
    .into_iter()
    .max()
    .unwrap_or_default();
    let upload_is_slow = stage_timings.upload_frame >= RENDER_SLOW_PATH_UPLOAD_LOG_THRESHOLD;
    let total_is_slow = total >= RENDER_SLOW_PATH_TOTAL_LOG_THRESHOLD;
    let lag_is_slow = lag_ms >= RENDER_SLOW_PATH_LAG_LOG_THRESHOLD_MS;
    if !upload_is_slow
        && !total_is_slow
        && !(lag_is_slow && slowest_stage >= Duration::from_millis(1))
    {
        return;
    }
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
        "renderer_present_slow",
        format!(
            "total_ms={:.2} upload_ms={:.2} acquire_ms={:.2} encode_submit_ms={:.2} present_ms={:.2} lag_ms={:.2} pts={:.3?}",
            total.as_secs_f64() * 1000.0,
            stage_timings.upload_frame.as_secs_f64() * 1000.0,
            stage_timings.acquire_surface.as_secs_f64() * 1000.0,
            stage_timings.encode_and_submit.as_secs_f64() * 1000.0,
            stage_timings.present.as_secs_f64() * 1000.0,
            lag_ms,
            frame.map(QueuedFrame::pts_seconds),
        ),
    );
}

