use super::{DecodedVideoFrame, QueuedFrame, Renderer, VideoFrame, VideoScaleMode};
use ffmpeg_next::frame;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::media::error::MediaError;
use crate::app::media::playback::runtime::emit_debug;
use tauri::Manager;

use super::frame_queue::{pick_frame_for_present, submit_frame_to_queue};
use super::helpers::wait_for_render_signal;

pub(super) const FRAME_QUEUE_HARD_CAPACITY: usize = 6;
const FRAME_QUEUE_TARGET_BASE_REALTIME: usize = 1;
const FRAME_QUEUE_TARGET_BASE_NON_REALTIME: usize = 4;
const RENDER_LOOP_IDLE_TICK: Duration = Duration::from_millis(8);
const QUEUE_GROW_LAG_MS: f64 = 24.0;
const QUEUE_SHRINK_LAG_MS: f64 = 8.0;
const QUEUE_GROW_STREAK: u8 = 2;
const QUEUE_SHRINK_STREAK: u8 = 18;
const QUEUE_GROW_HOLD_AFTER_RESET_REALTIME: Duration = Duration::from_millis(3000);
const QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME: Duration = Duration::from_millis(350);

#[derive(Clone)]
pub struct RendererState {
    pub(super) inner: Arc<RendererInner>,
}

pub(super) struct RendererInner {
    pub(super) stop: AtomicBool,
    pub(super) renderer: Mutex<Option<Renderer>>,
    pub(super) app_handle: Mutex<Option<tauri::AppHandle>>,
    pub(super) queued_frames: Mutex<VecDeque<QueuedFrame>>,
    pub(super) last_queued_pts: Mutex<Option<f64>>,
    pub(super) clock: Mutex<ClockState>,
    pub(super) render_task_in_flight: AtomicBool,
    pub(super) pending_render: Mutex<bool>,
    pub(super) render_cv: Condvar,
    pub(super) frame_slot_cv: Condvar,
    pub(super) video_scale_mode: AtomicU8,
    pub(super) is_realtime_source: AtomicBool,
    pub(super) target_queue_capacity: AtomicUsize,
    pub(super) last_render_cost_micros: AtomicU64,
    pub(super) last_present_lag_ms_bits: AtomicU32,
    pub(super) last_presented_pts_bits: AtomicU64,
    pub(super) queue_tuning: Mutex<QueueTuningState>,
    pub(super) queue_growth_hold_until: Mutex<Option<Instant>>,
    pub(super) last_frame_queue_trace_at: Mutex<Option<Instant>>,
    pub(super) last_render_schedule_trace_at: Mutex<Option<Instant>>,
}

#[derive(Clone, Copy)]
pub(super) struct ClockState {
    pub(super) anchor_instant: Instant,
    pub(super) anchor_media_seconds: f64,
    pub(super) rate: f64,
}

#[derive(Default)]
pub(super) struct QueueTuningState {
    pub(super) late_present_streak: u8,
    pub(super) healthy_present_streak: u8,
}

impl RendererState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            inner: Arc::new(RendererInner {
                stop: AtomicBool::new(false),
                renderer: Mutex::new(None),
                app_handle: Mutex::new(None),
                queued_frames: Mutex::new(VecDeque::with_capacity(16)),
                last_queued_pts: Mutex::new(None),
                clock: Mutex::new(ClockState {
                    anchor_instant: now,
                    anchor_media_seconds: 0.0,
                    rate: 1.0,
                }),
                render_task_in_flight: AtomicBool::new(false),
                pending_render: Mutex::new(false),
                render_cv: Condvar::new(),
                frame_slot_cv: Condvar::new(),
                video_scale_mode: AtomicU8::new(VideoScaleMode::Contain.as_u8()),
                is_realtime_source: AtomicBool::new(false),
                target_queue_capacity: AtomicUsize::new(FRAME_QUEUE_TARGET_BASE_NON_REALTIME),
                last_render_cost_micros: AtomicU64::new(0),
                last_present_lag_ms_bits: AtomicU32::new(0f32.to_bits()),
                last_presented_pts_bits: AtomicU64::new(f64::NAN.to_bits()),
                queue_tuning: Mutex::new(QueueTuningState::default()),
                queue_growth_hold_until: Mutex::new(None),
                last_frame_queue_trace_at: Mutex::new(None),
                last_render_schedule_trace_at: Mutex::new(None),
            }),
        }
    }

    pub fn set_video_scale_mode(&self, mode: VideoScaleMode) {
        self.inner
            .video_scale_mode
            .store(mode.as_u8(), Ordering::Relaxed);
    }

    pub fn set_realtime_source(&self, is_realtime_source: bool) {
        self.inner
            .is_realtime_source
            .store(is_realtime_source, Ordering::Relaxed);
        self.inner
            .target_queue_capacity
            .store(base_target_queue_capacity(&self.inner), Ordering::Relaxed);
        if let Ok(mut tuning) = self.inner.queue_tuning.lock() {
            *tuning = QueueTuningState::default();
        }
    }

    pub fn start_render_loop(&self, app: &tauri::AppHandle) -> Result<(), String> {
        if let Ok(mut app_handle) = self.inner.app_handle.lock() {
            *app_handle = Some(app.clone());
        }
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;

        let renderer = pollster::block_on(Renderer::new(window))?;
        {
            let mut guard = self
                .inner
                .renderer
                .lock()
                .map_err(|_| MediaError::state_poisoned_lock("renderer state").to_string())?;
            *guard = Some(renderer);
        }
        let _ = app.run_on_main_thread({
            let inner = self.inner.clone();
            move || {
                if let Ok(mut guard) = inner.renderer.lock() {
                    if let Some(renderer) = guard.as_mut() {
                        let _ = renderer.render(None, true);
                    }
                }
            }
        });

        let app_handle = app.clone();
        let inner = self.inner.clone();
        thread::spawn(move || {
            // Present continuously to align cadence with display vsync. When no new video
            // frame is due, we'll still present the previously uploaded texture.
            while !inner.stop.load(Ordering::Relaxed) {
                let _timed_out = wait_for_render_signal(&inner, RENDER_LOOP_IDLE_TICK);
                if inner.render_task_in_flight.swap(true, Ordering::AcqRel) {
                    continue;
                }
                let scheduled_at = Instant::now();
                let _ = app_handle.run_on_main_thread({
                    let inner = inner.clone();
                    move || {
                        maybe_emit_render_schedule_delay(&inner, scheduled_at);
                        let now_media_seconds = {
                            let clock = match inner.clock.lock() {
                                Ok(guard) => *guard,
                                Err(_) => ClockState {
                                    anchor_instant: Instant::now(),
                                    anchor_media_seconds: 0.0,
                                    rate: 1.0,
                                },
                            };
                            let elapsed =
                                Instant::now().saturating_duration_since(clock.anchor_instant);
                            clock.anchor_media_seconds
                                + elapsed.as_secs_f64() * clock.rate.max(0.25)
                        };
                        let selection = pick_frame_for_present(&inner, now_media_seconds);
                        let frame = selection.frame;
                        if let Ok(mut guard) = inner.renderer.lock() {
                            if let Some(renderer) = guard.as_mut() {
                                let render_start = Instant::now();
                                renderer.set_video_scale_mode(VideoScaleMode::from_u8(
                                    inner.video_scale_mode.load(Ordering::Relaxed),
                                ));
                                let _ = renderer.render(frame.as_ref(), true);
                                let elapsed = render_start.elapsed();
                                inner.last_render_cost_micros.store(
                                    u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
                                    Ordering::Relaxed,
                                );
                                let lag_ms = frame
                                    .as_ref()
                                    .map(|f| {
                                        ((now_media_seconds - f.pts_seconds()).max(0.0) * 1000.0)
                                            as f32
                                    })
                                    .unwrap_or(0.0);
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

        Ok(())
    }

    pub fn update_clock(&self, media_seconds: f64, playback_rate: f64) {
        if let Ok(mut guard) = self.inner.clock.lock() {
            *guard = ClockState {
                anchor_instant: Instant::now(),
                anchor_media_seconds: media_seconds.max(0.0),
                rate: if playback_rate.is_finite() && playback_rate > 0.0 {
                    playback_rate.min(3.0)
                } else {
                    1.0
                },
            };
        }
    }

    pub fn reset_timeline(&self, media_seconds: f64, playback_rate: f64) {
        if let Ok(mut queue) = self.inner.queued_frames.lock() {
            while let Some(frame) = queue.pop_front() {
                recycle_frame(&self.inner, frame);
            }
        }
        self.inner.frame_slot_cv.notify_all();
        if let Ok(mut last_pts) = self.inner.last_queued_pts.lock() {
            *last_pts = None;
        }
        self.inner
            .last_presented_pts_bits
            .store(f64::NAN.to_bits(), Ordering::Relaxed);
        self.inner
            .target_queue_capacity
            .store(base_target_queue_capacity(&self.inner), Ordering::Relaxed);
        if let Ok(mut tuning) = self.inner.queue_tuning.lock() {
            *tuning = QueueTuningState::default();
        }
        if let Ok(mut hold_until) = self.inner.queue_growth_hold_until.lock() {
            // After seek/restart/pause-resume we still debounce reactive growth, but for
            // non-realtime playback the hold must stay short so heavy GOP/B-frame content can
            // rebuild a useful render lead immediately instead of starving at depth 1.
            *hold_until = Some(Instant::now() + queue_growth_hold_after_reset(&self.inner));
        }
        self.update_clock(media_seconds, playback_rate);
        if let Ok(mut pending) = self.inner.pending_render.lock() {
            *pending = true;
            self.inner.render_cv.notify_one();
        }
    }

    pub fn clear_surface(&self, app: &tauri::AppHandle) -> Result<(), String> {
        if let Ok(mut queue) = self.inner.queued_frames.lock() {
            while let Some(frame) = queue.pop_front() {
                recycle_frame(&self.inner, frame);
            }
        }
        self.inner.frame_slot_cv.notify_all();
        if let Ok(mut last_pts) = self.inner.last_queued_pts.lock() {
            *last_pts = None;
        }
        self.inner
            .last_presented_pts_bits
            .store(f64::NAN.to_bits(), Ordering::Relaxed);
        let inner = self.inner.clone();
        app.run_on_main_thread(move || {
            if let Ok(mut guard) = inner.renderer.lock() {
                if let Some(renderer) = guard.as_mut() {
                    renderer.clear_uploaded_frame();
                    let _ = renderer.render(None, true);
                }
            }
        })
        .map_err(|err| format!("clear renderer surface failed: {err}"))
    }

    pub fn can_accept_frame(&self) -> bool {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|q| q.len() < self.queue_capacity())
            .unwrap_or(true)
    }

    pub fn wait_for_frame_slot(
        &self,
        stop_flag: &std::sync::atomic::AtomicBool,
        timeout: Duration,
    ) -> Result<(), String> {
        let mut queue = self
            .inner
            .queued_frames
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("renderer queue"))?;
        while queue.len() >= self.queue_capacity() {
            if stop_flag.load(Ordering::Relaxed) {
                return Ok(());
            }
            let (guard, _) = self
                .inner
                .frame_slot_cv
                .wait_timeout(queue, timeout)
                .map_err(|_| MediaError::state_poisoned_lock("renderer frame slot wait"))?;
            queue = guard;
        }
        Ok(())
    }

    pub fn queue_depth(&self) -> usize {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub fn queued_pts_range(&self) -> (Option<f64>, Option<f64>) {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|queue| {
                let head = queue.front().map(QueuedFrame::pts_seconds).filter(|v| v.is_finite());
                let tail = queue.back().map(QueuedFrame::pts_seconds).filter(|v| v.is_finite());
                (head, tail)
            })
            .unwrap_or((None, None))
    }

    pub fn queue_capacity(&self) -> usize {
        let min_capacity = base_target_queue_capacity(&self.inner);
        let max_capacity = max_target_queue_capacity(&self.inner);
        self.inner
            .target_queue_capacity
            .load(Ordering::Relaxed)
            .clamp(min_capacity, max_capacity)
    }

    pub fn current_clock_seconds(&self) -> f64 {
        self.inner
            .clock
            .lock()
            .ok()
            .map(|clock| {
                let elapsed = Instant::now().saturating_duration_since(clock.anchor_instant);
                clock.anchor_media_seconds + elapsed.as_secs_f64() * clock.rate.max(0.25)
            })
            .unwrap_or(0.0)
    }

    pub fn last_presented_pts_seconds(&self) -> Option<f64> {
        let value = f64::from_bits(self.inner.last_presented_pts_bits.load(Ordering::Relaxed));
        value.is_finite().then_some(value)
    }

    pub fn last_submitted_pts_seconds(&self) -> Option<f64> {
        self.queued_pts_range().1.or_else(|| {
            self.inner
                .last_queued_pts
                .lock()
                .ok()
                .and_then(|value| *value)
                .filter(|value| value.is_finite())
        })
    }

    pub fn submit_frame(&self, frame: VideoFrame) {
        submit_frame_to_queue(&self.inner, QueuedFrame::Prepared(frame));
    }

    pub fn submit_decoded_frame(
        &self,
        frame: frame::Video,
        pts_seconds: f64,
        color_matrix: [[f32; 3]; 3],
        y_offset: f32,
        y_scale: f32,
        uv_offset: f32,
        uv_scale: f32,
    ) {
        submit_frame_to_queue(
            &self.inner,
            QueuedFrame::Decoded(DecodedVideoFrame {
                pts_seconds,
                frame,
                color_matrix,
                y_offset,
                y_scale,
                uv_offset,
                uv_scale,
            }),
        );
    }
}

fn maybe_emit_render_schedule_delay(inner: &RendererInner, scheduled_at: Instant) {
    let delay = Instant::now().saturating_duration_since(scheduled_at);
    if delay < Duration::from_millis(20) {
        return;
    }
    let Ok(mut last_trace_at) = inner.last_render_schedule_trace_at.lock() else {
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
        "render_schedule_delay",
        format!("main_thread_delay_ms={:.3}", delay.as_secs_f64() * 1000.0),
    );
}

pub(super) fn recycle_frame(_inner: &RendererInner, _frame: QueuedFrame) {
}

fn update_dynamic_queue_capacity(
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

fn base_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        FRAME_QUEUE_TARGET_BASE_NON_REALTIME
    }
}

fn max_target_queue_capacity(inner: &RendererInner) -> usize {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        FRAME_QUEUE_TARGET_BASE_REALTIME
    } else {
        FRAME_QUEUE_HARD_CAPACITY
    }
}

fn queue_growth_hold_after_reset(inner: &RendererInner) -> Duration {
    if inner.is_realtime_source.load(Ordering::Relaxed) {
        QUEUE_GROW_HOLD_AFTER_RESET_REALTIME
    } else {
        QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME
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
