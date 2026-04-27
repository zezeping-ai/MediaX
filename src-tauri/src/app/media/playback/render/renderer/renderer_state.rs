use super::{Renderer, VideoFrame, VideoScaleMode};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::media::error::MediaError;
use tauri::Manager;

use super::frame_queue::{pick_frame_for_present, submit_frame_to_queue};
use super::helpers::wait_for_render_signal;

pub(super) const FRAME_QUEUE_HARD_CAPACITY: usize = 6;
const FRAME_QUEUE_TARGET_BASE: usize = 3;
const RENDER_LOOP_IDLE_TICK: Duration = Duration::from_millis(8);
const QUEUE_GROW_LAG_MS: f64 = 24.0;
const QUEUE_SHRINK_LAG_MS: f64 = 8.0;
const QUEUE_GROW_STREAK: u8 = 2;
const QUEUE_SHRINK_STREAK: u8 = 18;

#[derive(Clone)]
pub struct RendererState {
    pub(super) inner: Arc<RendererInner>,
}

pub(super) struct RendererInner {
    pub(super) stop: AtomicBool,
    pub(super) renderer: Mutex<Option<Renderer>>,
    pub(super) queued_frames: Mutex<VecDeque<VideoFrame>>,
    pub(super) last_queued_pts: Mutex<Option<f64>>,
    pub(super) clock: Mutex<ClockState>,
    pub(super) render_task_in_flight: AtomicBool,
    pub(super) pending_render: Mutex<bool>,
    pub(super) render_cv: Condvar,
    pub(super) video_scale_mode: AtomicU8,
    pub(super) target_queue_capacity: AtomicUsize,
    pub(super) last_render_cost_micros: AtomicU64,
    pub(super) last_present_lag_ms_bits: AtomicU32,
    pub(super) last_presented_pts_bits: AtomicU64,
    pub(super) queue_tuning: Mutex<QueueTuningState>,
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
                video_scale_mode: AtomicU8::new(VideoScaleMode::Contain.as_u8()),
                target_queue_capacity: AtomicUsize::new(FRAME_QUEUE_TARGET_BASE),
                last_render_cost_micros: AtomicU64::new(0),
                last_present_lag_ms_bits: AtomicU32::new(0f32.to_bits()),
                last_presented_pts_bits: AtomicU64::new(f64::NAN.to_bits()),
                queue_tuning: Mutex::new(QueueTuningState::default()),
            }),
        }
    }

    pub fn set_video_scale_mode(&self, mode: VideoScaleMode) {
        self.inner
            .video_scale_mode
            .store(mode.as_u8(), Ordering::Relaxed);
    }

    pub fn start_render_loop(&self, app: &tauri::AppHandle) -> Result<(), String> {
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
                let _ = app_handle.run_on_main_thread({
                    let inner = inner.clone();
                    move || {
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
                                    .map(|f| ((now_media_seconds - f.pts_seconds).max(0.0) * 1000.0) as f32)
                                    .unwrap_or(0.0);
                                inner
                                    .last_present_lag_ms_bits
                                    .store(lag_ms.to_bits(), Ordering::Relaxed);
                                let presented_pts = frame
                                    .as_ref()
                                    .map(|value| value.pts_seconds)
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
            queue.clear();
        }
        if let Ok(mut last_pts) = self.inner.last_queued_pts.lock() {
            *last_pts = None;
        }
        self.inner
            .last_presented_pts_bits
            .store(f64::NAN.to_bits(), Ordering::Relaxed);
        self.inner
            .target_queue_capacity
            .store(FRAME_QUEUE_TARGET_BASE, Ordering::Relaxed);
        if let Ok(mut tuning) = self.inner.queue_tuning.lock() {
            *tuning = QueueTuningState::default();
        }
        self.update_clock(media_seconds, playback_rate);
        if let Ok(mut pending) = self.inner.pending_render.lock() {
            *pending = true;
            self.inner.render_cv.notify_one();
        }
    }

    pub fn clear_surface(&self, app: &tauri::AppHandle) -> Result<(), String> {
        if let Ok(mut queue) = self.inner.queued_frames.lock() {
            queue.clear();
        }
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

    pub fn queue_depth(&self) -> usize {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|q| q.len())
            .unwrap_or(0)
    }

    pub fn queue_capacity(&self) -> usize {
        self.inner
            .target_queue_capacity
            .load(Ordering::Relaxed)
            .clamp(FRAME_QUEUE_TARGET_BASE, FRAME_QUEUE_HARD_CAPACITY)
    }

    pub fn last_presented_pts_seconds(&self) -> Option<f64> {
        let value = f64::from_bits(self.inner.last_presented_pts_bits.load(Ordering::Relaxed));
        value.is_finite().then_some(value)
    }

    pub fn last_submitted_pts_seconds(&self) -> Option<f64> {
        self.inner
            .last_queued_pts
            .lock()
            .ok()
            .and_then(|value| *value)
            .filter(|value| value.is_finite())
    }

    pub fn submit_frame(&self, frame: VideoFrame) {
        submit_frame_to_queue(&self.inner, frame);
    }
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
    let current_capacity = inner
        .target_queue_capacity
        .load(Ordering::Relaxed)
        .clamp(FRAME_QUEUE_TARGET_BASE, FRAME_QUEUE_HARD_CAPACITY);
    if has_presented_frame && (lag_ms >= QUEUE_GROW_LAG_MS || (lag_ms >= 12.0 && remaining_queue_depth == 0)) {
        tuning.late_present_streak = tuning.late_present_streak.saturating_add(1);
        tuning.healthy_present_streak = 0;
        if tuning.late_present_streak >= QUEUE_GROW_STREAK && current_capacity < FRAME_QUEUE_HARD_CAPACITY {
            inner
                .target_queue_capacity
                .store(current_capacity + 1, Ordering::Relaxed);
            tuning.late_present_streak = 0;
        }
        return;
    }
    if has_presented_frame && lag_ms <= QUEUE_SHRINK_LAG_MS && remaining_queue_depth >= 1 {
        tuning.healthy_present_streak = tuning.healthy_present_streak.saturating_add(1);
        tuning.late_present_streak = 0;
        if tuning.healthy_present_streak >= QUEUE_SHRINK_STREAK && current_capacity > FRAME_QUEUE_TARGET_BASE {
            inner
                .target_queue_capacity
                .store(current_capacity - 1, Ordering::Relaxed);
            tuning.healthy_present_streak = 0;
        }
        return;
    }
    tuning.late_present_streak = 0;
    tuning.healthy_present_streak = 0;
}
