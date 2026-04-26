use super::{emit_debug, percentile_from_sorted, METRICS_EMIT_INTERVAL_MS};
use crate::app::media::playback::render::video_frame::{
    detect_color_profile, video_frame_to_nv12_planes_from_yuv420p, ColorProfile,
};
use crate::app::media::playback::render::renderer::VideoFrame;
use ffmpeg_next::frame;
use std::time::{Duration, Instant};
use tauri::AppHandle;

#[derive(Default)]
pub(super) struct VideoIntegrityStats {
    dropped_hw_transfer: u64,
    dropped_nv12_extract: u64,
    dropped_scale: u64,
    color_profile_drift: u64,
    last_emit_instant: Option<Instant>,
    last_drift_log_instant: Option<Instant>,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct VideoIntegritySnapshot {
    pub dropped_hw_transfer: u64,
    pub dropped_nv12_extract: u64,
    pub dropped_scale: u64,
}

#[derive(Default)]
pub(crate) struct VideoFramePipeline {
    locked_color_profile: Option<ColorProfile>,
    integrity: VideoIntegrityStats,
    perf_window: VideoPerfWindow,
    first_frame_emitted: bool,
}

#[derive(Default)]
struct VideoPerfWindow {
    samples: u64,
    total_micros: u128,
    max_micros: u64,
    cost_samples_ms: Vec<f64>,
}

pub(crate) struct VideoPerfSnapshot {
    pub avg_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub samples: u64,
}

impl VideoFramePipeline {
    pub(super) fn record_frame_cost(&mut self, cost: Duration) {
        let micros = cost.as_micros();
        self.perf_window.samples = self.perf_window.samples.saturating_add(1);
        self.perf_window.total_micros = self.perf_window.total_micros.saturating_add(micros);
        self.perf_window.max_micros = self
            .perf_window
            .max_micros
            .max(u64::try_from(micros).unwrap_or(u64::MAX));
        self.perf_window.cost_samples_ms.push((micros as f64) / 1000.0);
    }

    pub(super) fn take_perf_snapshot(&mut self) -> Option<VideoPerfSnapshot> {
        if self.perf_window.samples == 0 {
            return None;
        }
        let samples = self.perf_window.samples;
        let avg_micros = (self.perf_window.total_micros as f64) / (samples as f64);
        let max_micros = self.perf_window.max_micros as f64;
        let mut sorted_costs_ms = self.perf_window.cost_samples_ms.clone();
        sorted_costs_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p50_ms = percentile_from_sorted(&sorted_costs_ms, 50.0);
        let p95_ms = percentile_from_sorted(&sorted_costs_ms, 95.0);
        let p99_ms = percentile_from_sorted(&sorted_costs_ms, 99.0);
        self.perf_window = VideoPerfWindow::default();
        Some(VideoPerfSnapshot {
            avg_ms: avg_micros / 1000.0,
            max_ms: max_micros / 1000.0,
            p50_ms,
            p95_ms,
            p99_ms,
            samples,
        })
    }

    pub(super) fn on_hw_transfer_failed(&mut self, app: &AppHandle, err: &str) {
        self.integrity.dropped_hw_transfer = self.integrity.dropped_hw_transfer.saturating_add(1);
        emit_debug(app, "hw_frame_transfer", format!("drop frame: {err}"));
        self.emit_integrity_if_needed(app);
    }

    pub(super) fn on_scale_failed(&mut self, app: &AppHandle, err: &str) {
        self.integrity.dropped_scale = self.integrity.dropped_scale.saturating_add(1);
        emit_debug(app, "video_scale", format!("drop frame: {err}"));
        self.emit_integrity_if_needed(app);
    }

    fn on_nv12_extract_failed(&mut self, app: &AppHandle, err: &str) {
        self.integrity.dropped_nv12_extract = self.integrity.dropped_nv12_extract.saturating_add(1);
        emit_debug(app, "nv12_extract", format!("drop frame: {err}"));
        self.emit_integrity_if_needed(app);
    }

    pub(super) fn resolve_color_profile(
        &mut self,
        app: &AppHandle,
        frame: &frame::Video,
    ) -> ColorProfile {
        let current_profile = detect_color_profile(frame);
        if let Some(locked) = self.locked_color_profile {
            if current_profile.color_matrix != locked.color_matrix {
                self.integrity.color_profile_drift =
                    self.integrity.color_profile_drift.saturating_add(1);
                let should_log_drift = self
                    .integrity
                    .last_drift_log_instant
                    .map(|last| last.elapsed() >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS))
                    .unwrap_or(true);
                if should_log_drift {
                    self.integrity.last_drift_log_instant = Some(Instant::now());
                    emit_debug(
                        app,
                        "color_profile_drift",
                        "frame color matrix changed; keep locked profile".to_string(),
                    );
                }
            }
            locked
        } else {
            self.locked_color_profile = Some(current_profile);
            emit_debug(app, "color_profile", "lock color profile from first frame".to_string());
            emit_debug(app, "video_frame_format", {
                let (sar_num, sar_den, interlaced, top_field_first) = unsafe {
                    let raw = &*frame.as_ptr();
                    (
                        raw.sample_aspect_ratio.num,
                        raw.sample_aspect_ratio.den,
                        raw.interlaced_frame != 0,
                        raw.top_field_first != 0,
                    )
                };
                let sar = if sar_num > 0 && sar_den > 0 {
                    format!("{sar_num}:{sar_den}")
                } else {
                    "n/a".to_string()
                };
                let dar = if sar_num > 0 && sar_den > 0 && frame.height() > 0 {
                    ((frame.width() as f64) * (sar_num as f64) / (sar_den as f64))
                        / (frame.height() as f64)
                } else if frame.height() > 0 {
                    (frame.width() as f64) / (frame.height() as f64)
                } else {
                    0.0
                };
                let scan_type = if interlaced {
                    if top_field_first {
                        "interlaced_tff"
                    } else {
                        "interlaced_bff"
                    }
                } else {
                    "progressive"
                };
                format!(
                    "pix_fmt={:?} color_space={:?} color_range={:?} sar={} dar≈{:.3} scan={} size={}x{}",
                    frame.format(),
                    frame.color_space(),
                    frame.color_range(),
                    sar,
                    dar,
                    scan_type,
                    frame.width(),
                    frame.height()
                )
            });
            current_profile
        }
    }

    pub(super) fn frame_to_renderer(
        &mut self,
        app: &AppHandle,
        frame: &frame::Video,
        pts: f64,
    ) -> Option<VideoFrame> {
        if !self.first_frame_emitted {
            self.first_frame_emitted = true;
            emit_debug(
                app,
                "first_frame",
                format!(
                    "first frame ready pts={:.3}s size={}x{} fmt={:?}",
                    pts.max(0.0),
                    frame.width(),
                    frame.height(),
                    frame.format(),
                ),
            );
        }
        let profile = self.resolve_color_profile(app, frame);
        let render_frame =
            match video_frame_to_nv12_planes_from_yuv420p(frame, Some(pts), Some(profile)) {
                Ok(frame) => frame,
                Err(err) => {
                    self.on_nv12_extract_failed(app, &err);
                    return None;
                }
            };
        self.emit_integrity_if_needed(app);
        Some(render_frame)
    }

    fn emit_integrity_if_needed(&mut self, app: &AppHandle) {
        let now = Instant::now();
        let should_emit = self
            .integrity
            .last_emit_instant
            .map(|last| {
                now.saturating_duration_since(last)
                    >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS)
            })
            .unwrap_or(true);
        if !should_emit {
            return;
        }
        self.integrity.last_emit_instant = Some(now);
        emit_debug(
            app,
            "video_integrity",
            format!(
                "drops(hw_transfer={}, scale={}, nv12_extract={}) color_profile_drift={}",
                self.integrity.dropped_hw_transfer,
                self.integrity.dropped_scale,
                self.integrity.dropped_nv12_extract,
                self.integrity.color_profile_drift
            ),
        );
    }

    pub(crate) fn integrity_snapshot(&self) -> VideoIntegritySnapshot {
        VideoIntegritySnapshot {
            dropped_hw_transfer: self.integrity.dropped_hw_transfer,
            dropped_nv12_extract: self.integrity.dropped_nv12_extract,
            dropped_scale: self.integrity.dropped_scale,
        }
    }
}
