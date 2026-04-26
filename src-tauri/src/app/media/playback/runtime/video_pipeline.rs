use super::{
    emit_debug, emit_telemetry_payloads, write_latest_stream_position, METRICS_EMIT_INTERVAL_MS,
};
use crate::app::media::playback::events::MediaTelemetryPayload;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::render::renderer::{RendererState, VideoFrame};
use crate::app::media::playback::render::video_frame::{
    detect_color_profile, ensure_scaler, transfer_hw_frame_if_needed,
    video_frame_to_nv12_planes_from_yuv420p, ColorProfile, ScalerSpec,
};
use crate::app::media::playback::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::state::MediaState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::ffi;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use tauri::{AppHandle, Manager};

#[derive(Default)]
pub(super) struct VideoIntegrityStats {
    dropped_hw_transfer: u64,
    dropped_nv12_extract: u64,
    color_profile_drift: u64,
    last_emit_instant: Option<Instant>,
    last_drift_log_instant: Option<Instant>,
}

#[derive(Default)]
pub(super) struct VideoFramePipeline {
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

pub(super) struct VideoPerfSnapshot {
    pub avg_ms: f64,
    pub max_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub samples: u64,
}

struct ProcessMetricsSnapshot {
    cpu_percent: f32,
    memory_mb: f64,
}

pub(super) struct ProcessMetricsSampler {
    system: System,
    pid: Pid,
}

pub(super) struct DrainFramesContext<'a> {
    pub app: &'a AppHandle,
    pub renderer: &'a RendererState,
    pub decoder: &'a mut ffmpeg::decoder::Video,
    pub video_time_base: ffmpeg::Rational,
    pub scaler: &'a mut Option<ScalingContext>,
    pub duration_seconds: f64,
    pub output_width: u32,
    pub output_height: u32,
    pub stop_flag: &'a std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub playback_clock: &'a mut PlaybackClock,
    pub last_progress_emit: &'a mut Instant,
    pub current_position_seconds: &'a mut f64,
    pub audio_clock: Option<AudioClock>,
    pub audio_queue_depth_sources: Option<usize>,
    pub active_seek_target_seconds: &'a mut Option<f64>,
    pub last_video_pts_seconds: &'a mut Option<f64>,
    pub fps_window: &'a mut FpsWindow,
    pub frame_pipeline: &'a mut VideoFramePipeline,
    pub process_metrics: &'a mut ProcessMetricsSampler,
    pub audio_allowed_lead_seconds: f64,
    pub network_read_bps: Option<f64>,
    pub media_required_bps: Option<f64>,
    pub video_ts_window_start: &'a mut Instant,
    pub video_ts_samples: &'a mut u64,
    pub video_pts_missing: &'a mut u64,
    pub video_pts_backtrack: &'a mut u64,
    pub video_pts_jitter_abs_sum_ms: &'a mut f64,
    pub video_pts_jitter_max_ms: &'a mut f64,
    pub video_frame_type_window_start: &'a mut Instant,
    pub video_frame_type_i: &'a mut u64,
    pub video_frame_type_p: &'a mut u64,
    pub video_frame_type_b: &'a mut u64,
    pub video_frame_type_other: &'a mut u64,
    pub stream_generation: u32,
}

impl ProcessMetricsSampler {
    pub(super) fn new() -> Self {
        let refresh = RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing());
        let mut sampler = Self {
            system: System::new_with_specifics(refresh),
            pid: Pid::from_u32(std::process::id()),
        };
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        sampler.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sampler.pid]),
            true,
            refresh,
        );
        std::thread::sleep(Duration::from_millis(120));
        sampler.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sampler.pid]),
            true,
            refresh,
        );
        sampler
    }

    fn sample(&mut self) -> Option<ProcessMetricsSnapshot> {
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            refresh,
        );
        let process = self.system.process(self.pid)?;
        let memory_mb = (process.memory() as f64) / (1024.0 * 1024.0);
        Some(ProcessMetricsSnapshot {
            cpu_percent: process.cpu_usage(),
            memory_mb,
        })
    }
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
        self.perf_window
            .cost_samples_ms
            .push((micros as f64) / 1000.0);
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
            emit_debug(
                app,
                "color_profile",
                "lock color profile from first frame".to_string(),
            );
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
                "drops(hw_transfer={}, nv12_extract={}) color_profile_drift={}",
                self.integrity.dropped_hw_transfer,
                self.integrity.dropped_nv12_extract,
                self.integrity.color_profile_drift
            ),
        );
    }
}

pub(super) fn drain_frames(ctx: &mut DrainFramesContext<'_>) -> Result<(), String> {
    let mut decoded = frame::Video::empty();
    while let Ok(()) = ctx.decoder.receive_frame(&mut decoded) {
        let frame_cost_start = Instant::now();
        if !ctx
            .app
            .state::<MediaState>()
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let pict_type = unsafe { (*decoded.as_ptr()).pict_type };
        if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_I {
            *ctx.video_frame_type_i = ctx.video_frame_type_i.saturating_add(1);
        } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_P {
            *ctx.video_frame_type_p = ctx.video_frame_type_p.saturating_add(1);
        } else if pict_type == ffi::AVPictureType::AV_PICTURE_TYPE_B {
            *ctx.video_frame_type_b = ctx.video_frame_type_b.saturating_add(1);
        } else {
            *ctx.video_frame_type_other = ctx.video_frame_type_other.saturating_add(1);
        }
        let hinted_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), ctx.video_time_base);
        *ctx.video_ts_samples = ctx.video_ts_samples.saturating_add(1);
        if decoded.pts().is_none() {
            *ctx.video_pts_missing = ctx.video_pts_missing.saturating_add(1);
        }
        let hinted_valid = hinted_seconds.filter(|v| v.is_finite() && *v >= 0.0);
        if let (Some(target), Some(hint)) = (*ctx.active_seek_target_seconds, hinted_valid) {
            if hint + 0.03 < target {
                continue;
            }
            *ctx.active_seek_target_seconds = None;
        }

        while !ctx.renderer.can_accept_frame() {
            if ctx.stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(
                super::video_pipeline::RENDER_BACKPRESSURE_SLEEP_MS,
            ));
        }

        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(err) => {
                ctx.frame_pipeline.on_hw_transfer_failed(ctx.app, &err);
                continue;
            }
        };
        ensure_scaler(
            ctx.scaler,
            ScalerSpec {
                src_format: frame_for_scale.format(),
                src_width: frame_for_scale.width(),
                src_height: frame_for_scale.height(),
                dst_format: format::pixel::Pixel::YUV420P,
                dst_width: ctx.output_width,
                dst_height: ctx.output_height,
                flags: Flags::BILINEAR,
            },
        )?;
        let mut nv12_frame = frame::Video::empty();
        if let Some(scaler) = ctx.scaler.as_mut() {
            scaler
                .run(&frame_for_scale, &mut nv12_frame)
                .map_err(|err| format!("scale frame failed: {err}"))?;
        }
        let _ = ctx
            .frame_pipeline
            .resolve_color_profile(ctx.app, &nv12_frame);
        let audio_now_seconds = ctx.audio_clock.map(|clock| clock.now_seconds());
        let position_seconds = ctx.playback_clock.tick(
            hinted_seconds,
            audio_now_seconds,
            ctx.audio_queue_depth_sources,
            ctx.audio_allowed_lead_seconds,
        );
        let estimated_pts = hinted_valid.unwrap_or_else(|| {
            if let Some(prev) = *ctx.last_video_pts_seconds {
                prev + ctx.playback_clock.frame_duration.as_secs_f64()
            } else {
                position_seconds.max(0.0)
            }
        });
        if let Some(prev) = *ctx.last_video_pts_seconds {
            let gap = estimated_pts - prev;
            let expected = ctx.playback_clock.frame_duration.as_secs_f64();
            if gap < 0.0 {
                *ctx.video_pts_backtrack = ctx.video_pts_backtrack.saturating_add(1);
            }
            if expected > 0.0 {
                let jitter_ms = ((gap - expected).abs()) * 1000.0;
                *ctx.video_pts_jitter_abs_sum_ms += jitter_ms;
                if jitter_ms > *ctx.video_pts_jitter_max_ms {
                    *ctx.video_pts_jitter_max_ms = jitter_ms;
                }
            }
            if gap.is_finite() && gap > expected * 1.8 {
                emit_debug(
                    ctx.app,
                    "video_gap",
                    format!("detected frame pts gap={gap:.3}s expected~{expected:.3}s"),
                );
            }
        }
        *ctx.last_video_pts_seconds = Some(estimated_pts);
        *ctx.current_position_seconds = if ctx.duration_seconds > 0.0 {
            position_seconds.min(ctx.duration_seconds)
        } else {
            position_seconds
        };
        write_latest_stream_position(
            &ctx.app.state::<MediaState>(),
            *ctx.current_position_seconds,
        )?;
        ctx.renderer.update_clock(
            *ctx.current_position_seconds,
            ctx.playback_clock.playback_rate(),
        );
        let Some(render_frame) =
            ctx.frame_pipeline
                .frame_to_renderer(ctx.app, &nv12_frame, estimated_pts)
        else {
            continue;
        };
        if !ctx
            .app
            .state::<MediaState>()
            .stream
            .is_generation_current(ctx.stream_generation)
        {
            return Ok(());
        }
        ctx.renderer.submit_frame(render_frame);
        ctx.frame_pipeline
            .record_frame_cost(frame_cost_start.elapsed());
        if let Some(render_fps) = ctx.fps_window.record_frame_and_compute() {
            let perf_snapshot = ctx.frame_pipeline.take_perf_snapshot();
            let process_snapshot = ctx.process_metrics.sample();
            let renderer_metrics = ctx.renderer.metrics_snapshot();
            emit_debug(ctx.app, "video_fps", format!("render_fps={render_fps:.2}"));
            let audio_now = ctx.audio_clock.map(|clock| clock.now_seconds());
            let audio_drift = audio_now.map(|a| estimated_pts - a);
            emit_debug(
                ctx.app,
                "av_sync",
                format!(
                    "a_minus_v={:.3}ms audio_clock={} video_pts={:.3}s queue_depth={}",
                    audio_drift.unwrap_or(0.0) * 1000.0,
                    audio_now
                        .map(|v| format!("{v:.3}s"))
                        .unwrap_or_else(|| "n/a".to_string()),
                    estimated_pts.max(0.0),
                    renderer_metrics.queue_depth
                ),
            );
            emit_debug(
                ctx.app,
                "video_pipeline",
                format!(
                    "pts={:.3}s queue_depth={} clock={:.3}s rate={:.2} output={}x{} decode_avg={:.2}ms decode_max={:.2}ms decode_p50={:.2}ms decode_p95={:.2}ms decode_p99={:.2}ms samples={}",
                    estimated_pts.max(0.0),
                    renderer_metrics.queue_depth,
                    *ctx.current_position_seconds,
                    ctx.playback_clock.playback_rate(),
                    ctx.output_width,
                    ctx.output_height,
                    perf_snapshot.as_ref().map(|v| v.avg_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.max_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.p50_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.p95_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.p99_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.samples).unwrap_or(0),
                ),
            );
            emit_debug(
                ctx.app,
                "decode_cost_quantiles",
                format!(
                    "p50={:.3}ms p95={:.3}ms p99={:.3}ms avg={:.3}ms max={:.3}ms samples={}",
                    perf_snapshot.as_ref().map(|v| v.p50_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.p95_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.p99_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.avg_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.max_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.samples).unwrap_or(0),
                ),
            );
            let ts_elapsed = ctx.video_ts_window_start.elapsed();
            if ts_elapsed >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
                let samples = (*ctx.video_ts_samples).max(1);
                let missing_ratio = (*ctx.video_pts_missing as f64) * 100.0 / (samples as f64);
                let avg_jitter_ms = *ctx.video_pts_jitter_abs_sum_ms / (samples as f64);
                emit_debug(
                    ctx.app,
                    "video_timestamps",
                    format!(
                        "samples={} pts_missing={:.2}% backtrack={} jitter_avg={:.3}ms jitter_max={:.3}ms",
                        samples, missing_ratio, *ctx.video_pts_backtrack, avg_jitter_ms, *ctx.video_pts_jitter_max_ms
                    ),
                );
                *ctx.video_ts_window_start = Instant::now();
                *ctx.video_ts_samples = 0;
                *ctx.video_pts_missing = 0;
                *ctx.video_pts_backtrack = 0;
                *ctx.video_pts_jitter_abs_sum_ms = 0.0;
                *ctx.video_pts_jitter_max_ms = 0.0;
            }
            let frame_type_elapsed = ctx.video_frame_type_window_start.elapsed();
            if frame_type_elapsed >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS) {
                let total = *ctx.video_frame_type_i
                    + *ctx.video_frame_type_p
                    + *ctx.video_frame_type_b
                    + *ctx.video_frame_type_other;
                if total > 0 {
                    emit_debug(
                        ctx.app,
                        "video_frame_types",
                        format!(
                            "I={:.1}% P={:.1}% B={:.1}% other={:.1}% samples={}",
                            (*ctx.video_frame_type_i as f64) * 100.0 / (total as f64),
                            (*ctx.video_frame_type_p as f64) * 100.0 / (total as f64),
                            (*ctx.video_frame_type_b as f64) * 100.0 / (total as f64),
                            (*ctx.video_frame_type_other as f64) * 100.0 / (total as f64),
                            total
                        ),
                    );
                }
                *ctx.video_frame_type_window_start = Instant::now();
                *ctx.video_frame_type_i = 0;
                *ctx.video_frame_type_p = 0;
                *ctx.video_frame_type_b = 0;
                *ctx.video_frame_type_other = 0;
            }
            emit_telemetry_payloads(
                ctx.app,
                MediaTelemetryPayload {
                    source_fps: 1.0 / ctx.playback_clock.frame_duration.as_secs_f64().max(1e-6),
                    render_fps,
                    queue_depth: renderer_metrics.queue_depth,
                    clock_seconds: *ctx.current_position_seconds,
                    current_video_pts_seconds: Some(estimated_pts.max(0.0)),
                    current_audio_clock_seconds: audio_now,
                    current_frame_type: Some(picture_type_label(pict_type).to_string()),
                    current_frame_width: Some(nv12_frame.width()),
                    current_frame_height: Some(nv12_frame.height()),
                    playback_rate: Some(ctx.playback_clock.playback_rate()),
                    network_read_bytes_per_second: ctx.network_read_bps,
                    media_required_bytes_per_second: ctx.media_required_bps,
                    network_sustain_ratio: match (ctx.network_read_bps, ctx.media_required_bps) {
                        (Some(read_bps), Some(required_bps)) if required_bps > 0.0 => {
                            Some((read_bps / required_bps).max(0.0))
                        }
                        _ => None,
                    },
                    audio_drift_seconds: audio_drift,
                    video_pts_gap_seconds: ctx
                        .last_video_pts_seconds
                        .as_ref()
                        .map(|prev| (estimated_pts - prev).max(0.0)),
                    seek_settle_ms: None,
                    decode_avg_frame_cost_ms: perf_snapshot.as_ref().map(|v| v.avg_ms),
                    decode_max_frame_cost_ms: perf_snapshot.as_ref().map(|v| v.max_ms),
                    decode_samples: perf_snapshot.as_ref().map(|v| v.samples),
                    process_cpu_percent: process_snapshot.as_ref().map(|v| v.cpu_percent),
                    process_memory_mb: process_snapshot.as_ref().map(|v| v.memory_mb),
                    gpu_queue_depth: Some(renderer_metrics.queue_depth),
                    gpu_queue_capacity: Some(renderer_metrics.queue_capacity),
                    gpu_queue_utilization: Some(
                        (renderer_metrics.queue_depth as f64)
                            / (renderer_metrics.queue_capacity.max(1) as f64),
                    ),
                    render_estimated_cost_ms: Some(renderer_metrics.last_render_cost_ms),
                    render_present_lag_ms: Some(renderer_metrics.last_present_lag_ms),
                },
            );
        }
        if ctx.last_progress_emit.elapsed() >= Duration::from_millis(200) {
            update_playback_progress(
                ctx.app,
                *ctx.current_position_seconds,
                ctx.duration_seconds,
                false,
            )?;
            *ctx.last_progress_emit = Instant::now();
        }
    }
    Ok(())
}

fn picture_type_label(pict_type: ffi::AVPictureType) -> &'static str {
    match pict_type {
        ffi::AVPictureType::AV_PICTURE_TYPE_I => "I",
        ffi::AVPictureType::AV_PICTURE_TYPE_P => "P",
        ffi::AVPictureType::AV_PICTURE_TYPE_B => "B",
        ffi::AVPictureType::AV_PICTURE_TYPE_S => "S",
        ffi::AVPictureType::AV_PICTURE_TYPE_SI => "SI",
        ffi::AVPictureType::AV_PICTURE_TYPE_SP => "SP",
        ffi::AVPictureType::AV_PICTURE_TYPE_BI => "BI",
        _ => "Other",
    }
}

pub(super) fn percentile_from_sorted(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let p = percentile.clamp(0.0, 100.0) / 100.0;
    let pos = p * ((sorted.len() - 1) as f64);
    let lower = pos.floor() as usize;
    let upper = pos.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let weight = pos - (lower as f64);
    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}

pub const DECODE_LEAD_SLEEP_MS: u64 = 5;
pub const RENDER_BACKPRESSURE_SLEEP_MS: u64 = 2;
