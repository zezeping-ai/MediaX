mod clock;
mod queue_policy;
mod render_loop;

use super::frame_queue::{present_lead_seconds_for_queue, submit_frame_to_queue};
use super::playback_head::build_playback_heads;
use super::{
    DecodedVideoFrame, QueuedFrame, Renderer, VideoFrame, VideoPlaybackHeads, VideoScaleMode,
};
use ffmpeg_next::frame;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;

use crate::app::media::error::MediaError;

use clock::current_clock_seconds;
use queue_policy::{
    base_target_queue_capacity, max_target_queue_capacity, queue_growth_hold_after_reset,
};
use render_loop::spawn_render_loop_thread;

pub(super) const FRAME_QUEUE_HARD_CAPACITY: usize = 6;
pub(super) const FRAME_QUEUE_TARGET_BASE_REALTIME: usize = 1;
pub(super) const FRAME_QUEUE_TARGET_BASE_NON_REALTIME: usize = 4;
pub(super) const QUEUE_GROW_HOLD_AFTER_RESET_REALTIME: Duration = Duration::from_millis(3000);
pub(super) const QUEUE_GROW_HOLD_AFTER_RESET_NON_REALTIME: Duration = Duration::from_millis(350);

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
    pub(super) render_loop_wakeups: AtomicU64,
    pub(super) render_attempts: AtomicU64,
    pub(super) render_presents: AtomicU64,
    pub(super) render_uploads: AtomicU64,
    pub(super) queue_tuning: Mutex<QueueTuningState>,
    pub(super) queue_growth_hold_until: Mutex<Option<Instant>>,
    pub(super) current_surface_width: AtomicU32,
    pub(super) current_surface_height: AtomicU32,
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
                render_loop_wakeups: AtomicU64::new(0),
                render_attempts: AtomicU64::new(0),
                render_presents: AtomicU64::new(0),
                render_uploads: AtomicU64::new(0),
                queue_tuning: Mutex::new(QueueTuningState::default()),
                queue_growth_hold_until: Mutex::new(None),
                current_surface_width: AtomicU32::new(0),
                current_surface_height: AtomicU32::new(0),
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

    pub fn boost_queue_capacity(&self, min_capacity: usize, hold_duration: Duration) {
        if self.inner.is_realtime_source.load(Ordering::Relaxed) {
            return;
        }
        let capped_min_capacity = min_capacity
            .clamp(base_target_queue_capacity(&self.inner), max_target_queue_capacity(&self.inner));
        let current_capacity = self.inner.target_queue_capacity.load(Ordering::Relaxed);
        if current_capacity < capped_min_capacity {
            self.inner
                .target_queue_capacity
                .store(capped_min_capacity, Ordering::Relaxed);
        }
        if let Ok(mut hold_until) = self.inner.queue_growth_hold_until.lock() {
            *hold_until = Some(Instant::now() + hold_duration);
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
        self.inner
            .current_surface_width
            .store(renderer.config.width, Ordering::Relaxed);
        self.inner
            .current_surface_height
            .store(renderer.config.height, Ordering::Relaxed);
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

        spawn_render_loop_thread(app.clone(), self.inner.clone());
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

    pub fn queue_metrics_snapshot(&self) -> (usize, usize, Option<f64>, Option<f64>) {
        let (queue_depth, queued_head_pts_seconds, queued_tail_pts_seconds) = self.queued_pts_bounds();
        let queue_capacity = self.queue_capacity().max(queue_depth);
        (
            queue_depth,
            queue_capacity,
            queued_head_pts_seconds,
            queued_tail_pts_seconds,
        )
    }

    pub fn queued_pts_range(&self) -> (Option<f64>, Option<f64>) {
        let (_, head, tail) = self.queued_pts_bounds();
        (head, tail)
    }

    pub fn queue_capacity(&self) -> usize {
        let min_capacity = base_target_queue_capacity(&self.inner);
        let max_capacity = max_target_queue_capacity(&self.inner);
        self.inner
            .target_queue_capacity
            .load(Ordering::Relaxed)
            .clamp(min_capacity, max_capacity)
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (
            self.inner.current_surface_width.load(Ordering::Relaxed),
            self.inner.current_surface_height.load(Ordering::Relaxed),
        )
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

    pub fn playback_heads(&self) -> VideoPlaybackHeads {
        let (queued_head_pts, present_lead_seconds) = self
            .inner
            .queued_frames
            .lock()
            .ok()
            .map(|queue| {
                let head = queue.front().map(QueuedFrame::pts_seconds).filter(|v| v.is_finite());
                let present_lead = present_lead_seconds_for_queue(&queue);
                (head, Some(present_lead))
            })
            .unwrap_or((None, None));
        build_playback_heads(
            self.last_presented_pts_seconds(),
            queued_head_pts,
            present_lead_seconds,
            current_clock_seconds(&self.inner),
        )
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

    fn queued_pts_bounds(&self) -> (usize, Option<f64>, Option<f64>) {
        self.inner
            .queued_frames
            .lock()
            .ok()
            .map(|queue| {
                let head = queue.front().map(QueuedFrame::pts_seconds).filter(|v| v.is_finite());
                let tail = queue.back().map(QueuedFrame::pts_seconds).filter(|v| v.is_finite());
                (queue.len(), head, tail)
            })
            .unwrap_or((0, None, None))
    }
}

pub(super) fn recycle_frame(_inner: &RendererInner, _frame: QueuedFrame) {}

