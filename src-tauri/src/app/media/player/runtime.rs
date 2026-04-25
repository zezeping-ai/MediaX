use crate::app::media::player::decode_context::open_video_decode_context;
use crate::app::media::error::MediaError;
use crate::app::media::player::events::{
    MediaDebugPayload, MediaEventEnvelope, MediaMetadataPayload, MediaTelemetryPayload,
    MEDIA_DEBUG_EVENT, MEDIA_DEBUG_EVENT_V2, MEDIA_METADATA_EVENT, MEDIA_PLAYBACK_DEBUG_EVENT,
    MEDIA_PLAYBACK_METADATA_EVENT, MEDIA_PLAYBACK_TELEMETRY_EVENT, MEDIA_PROTOCOL_VERSION,
    MEDIA_TELEMETRY_EVENT_V2,
};
use crate::app::media::player::pts::timestamp_to_seconds;
use crate::app::media::player::runtime::audio::clamp_playback_rate;
use crate::app::media::player::runtime::clock::{AudioClock, FpsWindow, PlaybackClock};
use crate::app::media::player::runtime::progress::update_playback_progress;
use crate::app::media::player::renderer::RendererState;
use crate::app::media::player::state::{AudioControls, MediaState, TimingControls};
use crate::app::media::player::video_frame::{
    detect_color_profile, ensure_scaler, transfer_hw_frame_if_needed,
    video_frame_to_nv12_planes_from_yuv420p, ColorProfile,
};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::format::sample::Type as SampleType;
use ffmpeg_next::frame;
use ffmpeg_next::media::Type;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use rodio::{buffer::SamplesBuffer, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use tauri::Manager;
use tauri::{AppHandle, Emitter};

const MAX_EMIT_FPS: u32 = 60;
const METRICS_EMIT_INTERVAL_MS: u64 = 1000;
const RATE_SWITCH_SETTLE_WINDOW_MS: u64 = 320;
const AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT: f64 = 0.02;
const AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE: f64 = 0.06;
const MAX_DECODE_LEAD_SECONDS_DEFAULT: f64 = 0.25;
const MAX_DECODE_LEAD_SECONDS_DURING_SETTLE: f64 = 0.45;

mod audio;
mod audio_pipeline;
mod clock;
mod progress;
mod seek_control;
mod session;
mod stream_control;
mod video_pipeline;

pub use stream_control::{
    read_latest_stream_position, start_decode_stream, stop_decode_stream,
    write_latest_stream_position,
};

fn emit_debug(app: &AppHandle, stage: &'static str, message: impl Into<String>) {
    let at_ms = unix_epoch_ms_now();
    let msg = message.into();
    let _ = app.emit(
        MEDIA_PLAYBACK_DEBUG_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_debug",
            request_id: None,
            emitted_at_ms: at_ms,
            payload: MediaDebugPayload {
                stage,
                message: msg.clone(),
                at_ms,
            },
        },
    );
    let _ = app.emit(
        MEDIA_DEBUG_EVENT,
        MediaDebugPayload {
            stage,
            message: msg.clone(),
            at_ms,
        },
    );
    let _ = app.emit(
        MEDIA_DEBUG_EVENT_V2,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "debug",
            request_id: None,
            emitted_at_ms: at_ms,
            payload: MediaDebugPayload {
                stage,
                message: msg,
                at_ms,
            },
        },
    );
}

fn unix_epoch_ms_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn emit_telemetry_payloads(app: &AppHandle, payload: MediaTelemetryPayload) {
    let emitted_at_ms = unix_epoch_ms_now();
    let _ = app.emit(
        MEDIA_PLAYBACK_TELEMETRY_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_telemetry",
            request_id: None,
            emitted_at_ms,
            payload: payload.clone(),
        },
    );
    let _ = app.emit(
        MEDIA_TELEMETRY_EVENT_V2,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "telemetry",
            request_id: None,
            emitted_at_ms,
            payload,
        },
    );
}

fn emit_metadata_payloads(app: &AppHandle, payload: MediaMetadataPayload) {
    let _ = app.emit(
        MEDIA_PLAYBACK_METADATA_EVENT,
        MediaEventEnvelope {
            protocol_version: MEDIA_PROTOCOL_VERSION,
            event_type: "playback_metadata",
            request_id: None,
            emitted_at_ms: unix_epoch_ms_now(),
            payload: payload.clone(),
        },
    );
    let _ = app.emit(MEDIA_METADATA_EVENT, payload);
}

pub(super) fn decode_and_emit_stream(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
) -> Result<(), String> {
    let media_state = app.state::<MediaState>();
    let (hw_mode, quality_mode) = {
        let playback = media_state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        (playback.hw_decode_mode(), playback.quality_mode())
    };
    emit_debug(app, "open", "open decode context");
    let mut video_ctx = open_video_decode_context(source, hw_mode, quality_mode)?;
    emit_debug(
        app,
        "decoder_ready",
        format!(
            "video: {}x{} fps={:.3} duration={:.3}s output={}x{}",
            video_ctx.decoder.width(),
            video_ctx.decoder.height(),
            video_ctx.fps_value,
            video_ctx.duration_seconds,
            video_ctx.output_width,
            video_ctx.output_height,
        ),
    );
    if let Some(video_stream) = video_ctx
        .input_ctx
        .streams()
        .find(|stream| stream.index() == video_ctx.video_stream_index)
    {
        let tb = video_stream.time_base();
        let avg = video_stream.avg_frame_rate();
        let r = video_stream.rate();
        let duration_ts = video_stream.duration();
        let duration_from_stream = if duration_ts > 0 {
            (duration_ts as f64) * f64::from(tb)
        } else {
            0.0
        };
        emit_debug(
            app,
            "video_stream",
            format!(
                "codec={:?} tb={}/{} avg_fps={:.3} nominal_fps={:.3} duration={:.3}s duration_ts={} start_ts={}",
                video_stream.parameters().id(),
                tb.numerator(),
                tb.denominator(),
                if avg.denominator() != 0 {
                    f64::from(avg.numerator()) / f64::from(avg.denominator())
                } else {
                    0.0
                },
                if r.denominator() != 0 {
                    f64::from(r.numerator()) / f64::from(r.denominator())
                } else {
                    0.0
                },
                duration_from_stream,
                duration_ts,
                video_stream.start_time()
            ),
        );
    }
    {
        let mut playback = media_state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.update_hw_decode_status(
            video_ctx.hw_decode_active,
            video_ctx.hw_decode_backend.clone(),
            None,
        );
    }
    let mut scaler: Option<ScalingContext> = None;
    let audio_stream_index = video_ctx
        .input_ctx
        .streams()
        .best(Type::Audio)
        .map(|stream| stream.index());
    emit_debug(
        app,
        "audio",
        match audio_stream_index {
            Some(i) => format!("audio stream index={i}"),
            None => "no audio stream".to_string(),
        },
    );
    let mut audio_pipeline = build_audio_pipeline(
        &video_ctx.input_ctx,
        audio_stream_index,
        audio_controls,
        timing_controls,
    )?;
    let mut last_applied_audio_rate: f32 = clamp_playback_rate(timing_controls.playback_rate());
    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            width: video_ctx.output_width,
            height: video_ctx.output_height,
            fps: video_ctx.fps_value,
            duration_seconds: video_ctx.duration_seconds,
        },
    );
    let mut playback_clock = PlaybackClock::new(
        video_ctx.fps_value,
        MAX_EMIT_FPS,
        0.0,
        timing_controls.clone(),
    );
    let mut last_progress_emit = Instant::now() - Duration::from_millis(250);
    let mut current_position_seconds = 0.0;
    let mut audio_clock: Option<AudioClock> = None;
    let mut audio_queue_depth_sources: Option<usize> = None;
    let mut active_seek_target_seconds: Option<f64> = None;
    let mut last_video_pts_seconds: Option<f64> = None;
    let mut rate_switch_settle_until: Option<Instant> = None;
    let mut fps_window = FpsWindow::default();
    let mut frame_pipeline = VideoFramePipeline::default();
    let mut process_metrics = ProcessMetricsSampler::new();
    renderer.reset_timeline(0.0, timing_controls.playback_rate() as f64);
    emit_debug(app, "running", "decode loop running");
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed; exiting decode loop");
            return Ok(());
        }

        // Apply speed changes immediately. Otherwise, rodio may keep playing already-queued samples
        // at the old speed until the next PCM append, which is especially noticeable when slowing down.
        if let Some(audio_state) = audio_pipeline.as_mut() {
            let next_rate = clamp_playback_rate(timing_controls.playback_rate());
            if (next_rate - last_applied_audio_rate).abs() > 1e-3 {
                rate_switch_settle_until =
                    Some(Instant::now() + Duration::from_millis(RATE_SWITCH_SETTLE_WINDOW_MS));
                if let Some(clock) = audio_clock.as_mut() {
                    // Preserve audio timeline continuity across rate changes; otherwise
                    // switching rate can make the clock jump backward/forward.
                    clock.rebase_rate(next_rate as f64);
                }
                audio_state.output.player.set_speed(next_rate);
                // If a lot of audio is already queued, changing speed can appear to "lag"
                // because the queued audio keeps playing at the old rate for a while.
                // Trade a small, bounded audio discontinuity for responsiveness.
                //
                // `len()` is the number of queued sources in rodio's player.
                // In practice, deep queues are most noticeable when slowing down (<1.0x).
                let queued_sources = audio_state.output.player.len();
                if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD && next_rate < 1.0 {
                    audio_state.output.player.clear();
                    audio_state.output.player.play();
                    audio_clock = None;
                    audio_queue_depth_sources = None;
                }
                last_applied_audio_rate = next_rate;
            }
        }

        // Keep decode near real-time. If we run far ahead (especially with a tiny video queue),
        // we can reach EOF quickly and the video will appear to "end" while audio is still draining.
        let in_rate_switch_settle = rate_switch_settle_until
            .map(|deadline| Instant::now() < deadline)
            .unwrap_or(false);
        let max_lead_seconds = if in_rate_switch_settle {
            MAX_DECODE_LEAD_SECONDS_DURING_SETTLE
        } else {
            MAX_DECODE_LEAD_SECONDS_DEFAULT
        };
        let audio_allowed_lead_seconds = if in_rate_switch_settle {
            AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE
        } else {
            AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT
        };
        let audio_now_seconds = audio_clock.as_ref().map(|clock| clock.now_seconds());
        if let (Some(video_pts), Some(audio_pts)) = (last_video_pts_seconds, audio_now_seconds)
        {
            let lead = video_pts - audio_pts;
            if lead.is_finite() && lead > max_lead_seconds {
                std::thread::sleep(Duration::from_millis(video_pipeline::DECODE_LEAD_SLEEP_MS));
                continue;
            }
        }
        if let Some(target_seconds) = seek_control::take_pending_seek_seconds(app)? {
            emit_debug(app, "seek", format!("apply seek to {target_seconds:.3}s"));
            seek_control::apply_seek_to_stream(
                &mut video_ctx.input_ctx,
                &mut video_ctx.decoder,
                target_seconds,
                &mut playback_clock,
                &mut current_position_seconds,
                audio_pipeline.as_mut(),
            )?;
            renderer.reset_timeline(target_seconds.max(0.0), timing_controls.playback_rate() as f64);
            // Seek creates a discontinuity. Drop previous audio clock/queue observations
            // so video pacing does not use stale pre-seek timing and cause stutter.
            audio_clock = None;
            audio_queue_depth_sources = None;
            active_seek_target_seconds = Some(target_seconds.max(0.0));
            last_video_pts_seconds = None;
            last_progress_emit = Instant::now() - Duration::from_millis(250);
            continue;
        }
        let mut packet = Packet::empty();
        match packet.read(&mut video_ctx.input_ctx) {
            Ok(_) => {
                if packet.stream() == video_ctx.video_stream_index {
                    video_ctx
                        .decoder
                        .send_packet(&packet)
                        .map_err(|err| format!("send packet failed: {err}"))?;
                    drain_frames(
                        app,
                        renderer,
                        &mut video_ctx.decoder,
                        video_ctx.video_time_base,
                        &mut scaler,
                        video_ctx.duration_seconds,
                        video_ctx.output_width,
                        video_ctx.output_height,
                        stop_flag,
                        &mut playback_clock,
                        &mut last_progress_emit,
                        &mut current_position_seconds,
                        audio_clock,
                        audio_queue_depth_sources,
                        &mut active_seek_target_seconds,
                        &mut last_video_pts_seconds,
                        &mut fps_window,
                        &mut frame_pipeline,
                        &mut process_metrics,
                        audio_allowed_lead_seconds,
                    )?;
                    continue;
                }
                if let Some(audio_state) = audio_pipeline.as_mut() {
                    if packet.stream() == audio_state.stream_index {
                        audio_state
                            .decoder
                            .send_packet(&packet)
                            .map_err(|err| format!("send audio packet failed: {err}"))?;
                        drain_audio_frames(
                            app,
                            audio_state,
                            stop_flag,
                            timing_controls,
                            &mut audio_clock,
                            &mut audio_queue_depth_sources,
                            &mut active_seek_target_seconds,
                        )?;
                    }
                    continue;
                }
            }
            Err(FfmpegError::Eof) => break,
            Err(_) => continue,
        }
    }
    video_ctx
        .decoder
        .send_eof()
        .map_err(|err| format!("send eof failed: {err}"))?;
    drain_frames(
        app,
        renderer,
        &mut video_ctx.decoder,
        video_ctx.video_time_base,
        &mut scaler,
        video_ctx.duration_seconds,
        video_ctx.output_width,
        video_ctx.output_height,
        stop_flag,
        &mut playback_clock,
        &mut last_progress_emit,
        &mut current_position_seconds,
        audio_clock,
        audio_queue_depth_sources,
        &mut active_seek_target_seconds,
        &mut last_video_pts_seconds,
        &mut fps_window,
        &mut frame_pipeline,
        &mut process_metrics,
        AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT,
    )?;
    if let Some(audio_state) = audio_pipeline.as_mut() {
        audio_state
            .decoder
            .send_eof()
            .map_err(|err| format!("send audio eof failed: {err}"))?;
        drain_audio_frames(
            app,
            audio_state,
            stop_flag,
            timing_controls,
            &mut audio_clock,
            &mut audio_queue_depth_sources,
            &mut active_seek_target_seconds,
        )?;
        if audio_state.stats.packets > 0 && audio_state.stats.decoded_frames == 0 {
            emit_debug(
                app,
                "audio_silent",
                format!(
                    "audio packets observed ({}) but no decoded audio frames produced",
                    audio_state.stats.packets
                ),
            );
        }
    }
    // EOF means the source is fully demuxed/decoded, but playback may still be in progress
    // (audio buffers draining, or video clock not yet reaching duration). Avoid ending early.
    let mut tail_position_seconds = current_position_seconds.max(0.0);
    let mut last_tail_tick = Instant::now();
    let mut last_tail_progress_emit = Instant::now() - Duration::from_millis(250);
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed during eof tail; exiting");
            return Ok(());
        }

        if let Some(audio_state) = audio_pipeline.as_mut() {
            let next_rate = clamp_playback_rate(timing_controls.playback_rate());
            if (next_rate - last_applied_audio_rate).abs() > 1e-3 {
                if let Some(clock) = audio_clock.as_mut() {
                    // Keep EOF tail pacing in sync with runtime rate changes.
                    clock.rebase_rate(next_rate as f64);
                }
                audio_state.output.player.set_speed(next_rate);
                let queued_sources = audio_state.output.player.len();
                if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD && next_rate < 1.0 {
                    audio_state.output.player.clear();
                    audio_state.output.player.play();
                }
                last_applied_audio_rate = next_rate;
            }
        }

        // If duration is unknown, fall back to finishing when audio drains (if any).
        let duration_seconds = video_ctx.duration_seconds.max(0.0);
        let rate = timing_controls.playback_rate().max(0.25) as f64;
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(last_tail_tick);
        last_tail_tick = now;
        if duration_seconds > 0.0 {
            tail_position_seconds =
                (tail_position_seconds + elapsed.as_secs_f64() * rate).min(duration_seconds);
        }
        renderer.update_clock(tail_position_seconds, rate);
        write_latest_stream_position(&app.state::<MediaState>(), tail_position_seconds)?;
        if last_tail_progress_emit.elapsed() >= Duration::from_millis(200) {
            update_playback_progress(app, tail_position_seconds, duration_seconds, false)?;
            last_tail_progress_emit = Instant::now();
        }

        let audio_done = audio_pipeline
            .as_ref()
            .map(|audio| audio.output.player.len() == 0)
            .unwrap_or(true);
        let duration_done = duration_seconds <= 0.0 || tail_position_seconds + 1e-3 >= duration_seconds;
        if audio_done && duration_done {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    update_playback_progress(app, tail_position_seconds, video_ctx.duration_seconds, true)?;
    Ok(())
}

fn drain_frames(
    app: &AppHandle,
    renderer: &RendererState,
    decoder: &mut ffmpeg::decoder::Video,
    video_time_base: ffmpeg::Rational,
    scaler: &mut Option<ScalingContext>,
    duration_seconds: f64,
    output_width: u32,
    output_height: u32,
    stop_flag: &Arc<AtomicBool>,
    playback_clock: &mut PlaybackClock,
    last_progress_emit: &mut Instant,
    current_position_seconds: &mut f64,
    audio_clock: Option<AudioClock>,
    audio_queue_depth_sources: Option<usize>,
    active_seek_target_seconds: &mut Option<f64>,
    last_video_pts_seconds: &mut Option<f64>,
    fps_window: &mut FpsWindow,
    frame_pipeline: &mut VideoFramePipeline,
    process_metrics: &mut ProcessMetricsSampler,
    audio_allowed_lead_seconds: f64,
) -> Result<(), String> {
    let mut decoded = frame::Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        let frame_cost_start = Instant::now();
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        let hinted_seconds = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), video_time_base);
        let hinted_valid = hinted_seconds.filter(|v| v.is_finite() && *v >= 0.0);
        if let (Some(target), Some(hint)) = (
            *active_seek_target_seconds,
            hinted_valid,
        ) {
            // FFmpeg seek usually lands on/near keyframe before target; drop preroll frames.
            if hint + 0.03 < target {
                continue;
            }
            *active_seek_target_seconds = None;
        }

        // Apply backpressure when the renderer is backed up. For "pro" playback quality,
        // prefer waiting briefly over dropping a large run of consecutive frames (which
        // feels like a slideshow).
        while !renderer.can_accept_frame() {
            if stop_flag.load(Ordering::Relaxed) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(video_pipeline::RENDER_BACKPRESSURE_SLEEP_MS));
        }

        let frame_for_scale = match transfer_hw_frame_if_needed(&decoded) {
            Ok(frame) => frame,
            Err(err) => {
                frame_pipeline.on_hw_transfer_failed(app, &err);
                continue;
            }
        };
        ensure_scaler(
            scaler,
            frame_for_scale.format(),
            frame_for_scale.width(),
            frame_for_scale.height(),
            format::pixel::Pixel::YUV420P,
            output_width,
            output_height,
            Flags::BILINEAR,
        )?;
        let mut nv12_frame = frame::Video::empty();
        if let Some(scaler) = scaler.as_mut() {
            scaler
                .run(&frame_for_scale, &mut nv12_frame)
                .map_err(|err| format!("scale frame failed: {err}"))?;
        }
        let _ = frame_pipeline.resolve_color_profile(app, &nv12_frame);
        let audio_now_seconds = audio_clock.map(|clock| clock.now_seconds());
        let position_seconds = playback_clock.tick(
            hinted_seconds,
            audio_now_seconds,
            audio_queue_depth_sources,
            audio_allowed_lead_seconds,
        );
        // Use stream PTS when available; otherwise estimate a monotonic PTS so we don't
        // mislabel far-ahead decoded frames as "due now" (which looks like fast-forward).
        let estimated_pts = hinted_valid.unwrap_or_else(|| {
            if let Some(prev) = *last_video_pts_seconds {
                prev + playback_clock.frame_duration.as_secs_f64()
            } else {
                position_seconds.max(0.0)
            }
        });
        if let Some(prev) = *last_video_pts_seconds {
            let gap = estimated_pts - prev;
            let expected = playback_clock.frame_duration.as_secs_f64();
            if gap.is_finite() && gap > expected * 1.8 {
                emit_debug(
                    app,
                    "video_gap",
                    format!("detected frame pts gap={gap:.3}s expected~{expected:.3}s"),
                );
            }
        }
        *last_video_pts_seconds = Some(estimated_pts);
        *current_position_seconds = if duration_seconds > 0.0 {
            position_seconds.min(duration_seconds)
        } else {
            position_seconds
        };
        write_latest_stream_position(&app.state::<MediaState>(), *current_position_seconds)?;
        renderer.update_clock(*current_position_seconds, playback_clock.playback_rate());
        let Some(render_frame) = frame_pipeline.frame_to_renderer(app, &nv12_frame, estimated_pts) else {
            continue;
        };
        renderer.submit_frame(render_frame);
        frame_pipeline.record_frame_cost(frame_cost_start.elapsed());
        if let Some(render_fps) = fps_window.record_frame_and_compute() {
            let perf_snapshot = frame_pipeline.take_perf_snapshot();
            let process_snapshot = process_metrics.sample();
            let renderer_metrics = renderer.metrics_snapshot();
            emit_debug(app, "video_fps", format!("render_fps={render_fps:.2}"));
            let audio_now = audio_clock.map(|clock| clock.now_seconds());
            let audio_drift = audio_now.map(|a| estimated_pts - a);
            emit_debug(
                app,
                "video_pipeline",
                format!(
                    "pts={:.3}s queue_depth={} clock={:.3}s rate={:.2} output={}x{} decode_avg={:.2}ms decode_max={:.2}ms samples={}",
                    estimated_pts.max(0.0),
                    renderer_metrics.queue_depth,
                    *current_position_seconds,
                    playback_clock.playback_rate(),
                    output_width,
                    output_height,
                    perf_snapshot.as_ref().map(|v| v.avg_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.max_ms).unwrap_or(0.0),
                    perf_snapshot.as_ref().map(|v| v.samples).unwrap_or(0),
                ),
            );
            emit_telemetry_payloads(
                app,
                MediaTelemetryPayload {
                    source_fps: 1.0 / playback_clock.frame_duration.as_secs_f64().max(1e-6),
                    render_fps,
                    queue_depth: renderer_metrics.queue_depth,
                    clock_seconds: *current_position_seconds,
                    audio_drift_seconds: audio_drift,
                    video_pts_gap_seconds: last_video_pts_seconds.map(|prev| (estimated_pts - prev).max(0.0)),
                    seek_settle_ms: None,
                    decode_avg_frame_cost_ms: perf_snapshot.as_ref().map(|v| v.avg_ms),
                    decode_max_frame_cost_ms: perf_snapshot.as_ref().map(|v| v.max_ms),
                    decode_samples: perf_snapshot.as_ref().map(|v| v.samples),
                    process_cpu_percent: process_snapshot.as_ref().map(|v| v.cpu_percent),
                    process_memory_mb: process_snapshot.as_ref().map(|v| v.memory_mb),
                    gpu_queue_depth: Some(renderer_metrics.queue_depth),
                    gpu_queue_capacity: Some(renderer_metrics.queue_capacity),
                    gpu_queue_utilization: Some(
                        (renderer_metrics.queue_depth as f64) / (renderer_metrics.queue_capacity.max(1) as f64),
                    ),
                    render_estimated_cost_ms: Some(renderer_metrics.last_render_cost_ms),
                    render_present_lag_ms: Some(renderer_metrics.last_present_lag_ms),
                },
            );
        }
        if last_progress_emit.elapsed() >= Duration::from_millis(200) {
            update_playback_progress(app, *current_position_seconds, duration_seconds, false)?;
            *last_progress_emit = Instant::now();
        }
    }
    Ok(())
}


struct AudioPipeline {
    stream_index: usize,
    decoder: ffmpeg::decoder::Audio,
    time_base: ffmpeg::Rational,
    resampler: ResamplingContext,
    output: AudioOutput,
    stats: AudioStats,
}

struct AudioOutput {
    _stream: MixerDeviceSink,
    player: Player,
    controls: Arc<AudioControls>,
    timing_controls: Arc<TimingControls>,
}

#[derive(Default)]
struct AudioStats {
    packets: u64,
    decoded_frames: u64,
    queued_samples: u64,
    last_debug_instant: Option<Instant>,
}

#[derive(Default)]
struct VideoIntegrityStats {
    dropped_hw_transfer: u64,
    dropped_nv12_extract: u64,
    color_profile_drift: u64,
    last_emit_instant: Option<Instant>,
    last_drift_log_instant: Option<Instant>,
}

#[derive(Default)]
struct VideoFramePipeline {
    locked_color_profile: Option<ColorProfile>,
    integrity: VideoIntegrityStats,
    perf_window: VideoPerfWindow,
}

#[derive(Default)]
struct VideoPerfWindow {
    samples: u64,
    total_micros: u128,
    max_micros: u64,
}

struct VideoPerfSnapshot {
    avg_ms: f64,
    max_ms: f64,
    samples: u64,
}

struct ProcessMetricsSnapshot {
    cpu_percent: f32,
    memory_mb: f64,
}

struct ProcessMetricsSampler {
    system: System,
    pid: Pid,
}

impl ProcessMetricsSampler {
    fn new() -> Self {
        let refresh = RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing());
        let mut sampler = Self {
            system: System::new_with_specifics(refresh),
            pid: Pid::from_u32(std::process::id()),
        };

        // `sysinfo` computes cpu_usage() based on deltas between refreshes.
        // Prime a baseline (and give it a short interval) so first samples are meaningful.
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        sampler
            .system
            .refresh_processes_specifics(ProcessesToUpdate::Some(&[sampler.pid]), true, refresh);
        std::thread::sleep(Duration::from_millis(120));
        sampler
            .system
            .refresh_processes_specifics(ProcessesToUpdate::Some(&[sampler.pid]), true, refresh);

        sampler
    }

    fn sample(&mut self) -> Option<ProcessMetricsSnapshot> {
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        self.system
            .refresh_processes_specifics(ProcessesToUpdate::Some(&[self.pid]), true, refresh);
        let process = self.system.process(self.pid)?;
        let memory_mb = (process.memory() as f64) / (1024.0 * 1024.0);
        Some(ProcessMetricsSnapshot {
            cpu_percent: process.cpu_usage(),
            memory_mb,
        })
    }
}

impl VideoFramePipeline {
    fn record_frame_cost(&mut self, cost: Duration) {
        let micros = cost.as_micros();
        self.perf_window.samples = self.perf_window.samples.saturating_add(1);
        self.perf_window.total_micros = self.perf_window.total_micros.saturating_add(micros);
        self.perf_window.max_micros = self
            .perf_window
            .max_micros
            .max(u64::try_from(micros).unwrap_or(u64::MAX));
    }

    fn take_perf_snapshot(&mut self) -> Option<VideoPerfSnapshot> {
        if self.perf_window.samples == 0 {
            return None;
        }
        let samples = self.perf_window.samples;
        let avg_micros = (self.perf_window.total_micros as f64) / (samples as f64);
        let max_micros = self.perf_window.max_micros as f64;
        self.perf_window = VideoPerfWindow::default();
        Some(VideoPerfSnapshot {
            avg_ms: avg_micros / 1000.0,
            max_ms: max_micros / 1000.0,
            samples,
        })
    }

    fn on_hw_transfer_failed(&mut self, app: &AppHandle, err: &str) {
        self.integrity.dropped_hw_transfer = self.integrity.dropped_hw_transfer.saturating_add(1);
        emit_debug(app, "hw_frame_transfer", format!("drop frame: {err}"));
        self.emit_integrity_if_needed(app);
    }

    fn on_nv12_extract_failed(&mut self, app: &AppHandle, err: &str) {
        self.integrity.dropped_nv12_extract = self.integrity.dropped_nv12_extract.saturating_add(1);
        emit_debug(app, "nv12_extract", format!("drop frame: {err}"));
        self.emit_integrity_if_needed(app);
    }

    fn resolve_color_profile(&mut self, app: &AppHandle, frame: &frame::Video) -> ColorProfile {
        let current_profile = detect_color_profile(frame);
        if let Some(locked) = self.locked_color_profile {
            if current_profile.color_matrix != locked.color_matrix {
                self.integrity.color_profile_drift = self.integrity.color_profile_drift.saturating_add(1);
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
            current_profile
        }
    }

    fn frame_to_renderer(
        &mut self,
        app: &AppHandle,
        frame: &frame::Video,
        pts: f64,
    ) -> Option<crate::app::media::player::renderer::VideoFrame> {
        let profile = self.resolve_color_profile(app, frame);
        let render_frame = match video_frame_to_nv12_planes_from_yuv420p(frame, Some(pts), Some(profile)) {
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
                now.saturating_duration_since(last) >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS)
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

impl AudioOutput {
    fn new(
        controls: Arc<AudioControls>,
        timing_controls: Arc<TimingControls>,
    ) -> Result<Self, String> {
        let mut stream = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("open default audio output failed: {err}"))?;
        stream.log_on_drop(false);
        let player = Player::connect_new(stream.mixer());
        let output = Self {
            _stream: stream,
            player,
            controls,
            timing_controls,
        };
        output.player.play();
        output.apply_controls();
        Ok(output)
    }

    fn apply_controls(&self) {
        let volume = if self.controls.muted() {
            0.0
        } else {
            self.controls.volume()
        };
        self.player.set_volume(volume);
        self.player
            .set_speed(clamp_playback_rate(self.timing_controls.playback_rate()));
    }

    fn append_pcm_i16(&self, sample_rate: u32, channels: u16, pcm: &[i16]) {
        let Some(channels) = NonZeroU16::new(channels) else {
            return;
        };
        let Some(sample_rate) = NonZeroU32::new(sample_rate) else {
            return;
        };
        if pcm.is_empty() {
            return;
        }
        self.apply_controls();
        let samples: Vec<f32> = pcm
            .iter()
            .map(|sample| (*sample as f32) / (i16::MAX as f32))
            .collect();
        self.player
            .append(SamplesBuffer::new(channels, sample_rate, samples));
    }
}

fn build_audio_pipeline(
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
) -> Result<Option<AudioPipeline>, String> {
    let Some(stream_index) = audio_stream_index else {
        return Ok(None);
    };
    let input_stream = input_ctx
        .streams()
        .find(|stream| stream.index() == stream_index)
        .ok_or_else(|| "audio stream index not found".to_string())?;
    let audio_context = codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|err| format!("audio decoder context failed: {err}"))?;
    let decoder = audio_context
        .decoder()
        .audio()
        .map_err(|err| format!("audio decoder create failed: {err}"))?;
    let channel_layout = if decoder.channel_layout().is_empty() {
        ChannelLayout::default(decoder.channels().into())
    } else {
        decoder.channel_layout()
    };
    let resampler = ResamplingContext::get(
        decoder.format(),
        channel_layout,
        decoder.rate(),
        ffmpeg::format::Sample::I16(SampleType::Packed),
        channel_layout,
        decoder.rate(),
    )
    .map_err(|err| format!("audio resampler create failed: {err}"))?;
    let output = AudioOutput::new(audio_controls.clone(), timing_controls.clone())?;
    Ok(Some(AudioPipeline {
        stream_index,
        decoder,
        time_base: input_stream.time_base(),
        resampler,
        output,
        stats: AudioStats::default(),
    }))
}

fn drain_audio_frames(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    audio_clock: &mut Option<AudioClock>,
    audio_queue_depth_sources: &mut Option<usize>,
    active_seek_target_seconds: &mut Option<f64>,
) -> Result<(), String> {
    audio_state.stats.packets = audio_state.stats.packets.saturating_add(1);
    let mut decoded = frame::Audio::empty();
    while audio_state.decoder.receive_frame(&mut decoded).is_ok() {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        audio_state.stats.decoded_frames = audio_state.stats.decoded_frames.saturating_add(1);
        let mut converted = frame::Audio::empty();
        audio_state
            .resampler
            .run(&decoded, &mut converted)
            .map_err(|err| format!("audio resample failed: {err}"))?;
        let channels = converted.channels().max(1) as usize;
        let samples_per_channel = converted.samples();
        let total_samples = samples_per_channel.saturating_mul(channels);
        if total_samples == 0 {
            continue;
        }
        let bytes_per_sample = std::mem::size_of::<i16>();
        let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
        let data = converted.data(0);
        if data.is_empty() {
            continue;
        }
        let clamped_bytes = expected_bytes.min(data.len());
        if clamped_bytes < bytes_per_sample {
            continue;
        }
        audio_state
            .output
            .player
            .set_speed(clamp_playback_rate(timing_controls.playback_rate()));
        if audio_state.output.player.is_paused() {
            audio_state.output.player.play();
            emit_debug(app, "audio_resume", "audio player resumed from paused state");
        }
        if let Some(seconds) = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), audio_state.time_base)
            .filter(|value| value.is_finite() && *value >= 0.0)
        {
            if let Some(target) = *active_seek_target_seconds {
                if seconds + 0.03 < target {
                    continue;
                }
                *active_seek_target_seconds = None;
            }
            if audio_clock.is_none() {
                *audio_clock = Some(AudioClock {
                    anchor_instant: Instant::now(),
                    anchor_media_seconds: seconds,
                    anchor_rate: timing_controls.playback_rate().max(0.25) as f64,
                });
            }
        }
        let pcm: Vec<i16> = data[..clamped_bytes]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        audio_state.stats.queued_samples = audio_state
            .stats
            .queued_samples
            .saturating_add(pcm.len() as u64);
        audio_state
            .output
            .append_pcm_i16(converted.rate(), converted.channels(), &pcm);
        *audio_queue_depth_sources = Some(audio_state.output.player.len());
        let now = Instant::now();
        let should_emit = audio_state
            .stats
            .last_debug_instant
            .map(|last| {
                now.saturating_duration_since(last) >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS)
            })
            .unwrap_or(true);
        if should_emit {
            audio_state.stats.last_debug_instant = Some(now);
            emit_debug(
                app,
                "audio_stats",
                format!(
                    "packets={} frames={} queued_samples={} rate={:.2} channels={} samples_per_ch={} bytes={} pts={}",
                    audio_state.stats.packets,
                    audio_state.stats.decoded_frames,
                    audio_state.stats.queued_samples,
                    timing_controls.playback_rate(),
                    channels,
                    samples_per_channel,
                    clamped_bytes,
                    audio_clock
                        .as_ref()
                        .map(|clock| clock.now_seconds())
                        .map(|v| format!("{v:.3}s"))
                        .unwrap_or_else(|| "n/a".to_string()),
                ),
            );
        }
    }
    Ok(())
}

