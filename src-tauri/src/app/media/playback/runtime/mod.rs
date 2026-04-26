use crate::app::media::error::MediaError;
use crate::app::media::playback::decode_context::{open_video_decode_context, VideoDecodeContext};
use crate::app::media::playback::events::{
    MediaDebugPayload, MediaEventEnvelope, MediaMetadataPayload, MediaTelemetryPayload,
    MEDIA_PLAYBACK_DEBUG_EVENT, MEDIA_PLAYBACK_METADATA_EVENT, MEDIA_PLAYBACK_TELEMETRY_EVENT,
    MEDIA_PROTOCOL_VERSION,
};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::playback::runtime::audio_pipeline::{
    build_audio_pipeline, drain_audio_frames, AudioPipeline,
};
use crate::app::media::playback::runtime::progress::update_playback_progress;
use crate::app::media::playback::runtime::session::{
    current_recording_target, update_cache_session_error, CacheRemuxWriter, DecodeLoopState,
};
use crate::app::media::playback::runtime::video_pipeline::{drain_frames, DrainFramesContext};
use crate::app::media::state::{AudioControls, MediaState, TimingControls};
use ffmpeg_next::codec;
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::context::Context as ScalingContext;
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
    read_latest_stream_position, start_decode_stream, stop_decode_stream_blocking,
    stop_decode_stream_non_blocking, write_latest_stream_position,
};

struct DecodeRuntime {
    video_ctx: VideoDecodeContext,
    scaler: Option<ScalingContext>,
    audio_pipeline: Option<AudioPipeline>,
    loop_state: DecodeLoopState,
    should_tail_eof: bool,
}

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
}

pub(super) fn decode_and_emit_stream(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
    stream_generation: u32,
) -> Result<(), String> {
    let mut runtime =
        create_decode_runtime(app, renderer, source, audio_controls, timing_controls)?;
    run_decode_loop(
        app,
        renderer,
        source,
        stop_flag,
        timing_controls,
        &mut runtime,
        stream_generation,
    )?;
    finish_decode_runtime(
        app,
        renderer,
        stop_flag,
        timing_controls,
        &mut runtime,
        stream_generation,
    )
}

fn create_decode_runtime(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
) -> Result<DecodeRuntime, String> {
    let is_live_m3u8 = source.to_ascii_lowercase().contains(".m3u8");
    let is_cache_recording_mp4 = {
        let lower = source.to_ascii_lowercase();
        lower.ends_with(".mp4") && lower.contains("mediax-cache-")
    };
    let should_tail_eof = is_live_m3u8 || is_cache_recording_mp4;
    let media_state = app.state::<MediaState>();
    let (hw_mode, quality_mode) = {
        let playback = media_state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        (playback.hw_decode_mode(), playback.quality_mode())
    };
    emit_debug(app, "open", "open decode context");
    let video_ctx = open_video_decode_context(source, hw_mode, quality_mode)?;
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
    {
        // SAFETY: decoder pointer is valid while `video_ctx.decoder` is alive; read-only access.
        let (profile, level, has_b_frames) = unsafe {
            let raw = &*video_ctx.decoder.as_ptr();
            (raw.profile, raw.level, raw.has_b_frames)
        };
        emit_debug(
            app,
            "video_codec_profile",
            format!(
                "codec={:?} profile={} level={} has_b_frames={}",
                video_ctx.decoder.id(),
                profile,
                level,
                has_b_frames
            ),
        );
    }
    if let Some(video_stream) = video_ctx
        .input_ctx
        .streams()
        .find(|stream| stream.index() == video_ctx.video_stream_index)
    {
        let container_name = video_ctx.input_ctx.format().name().to_string();
        emit_debug(
            app,
            "video_format",
            format!(
                "container={} codec={:?} pixel_fmt={:?}",
                container_name,
                video_stream.parameters().id(),
                video_ctx.decoder.format()
            ),
        );
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
            video_ctx.hw_decode_error.clone(),
        );
    }
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
    if let Some(audio_index) = audio_stream_index {
        if let Some(audio_stream) = video_ctx
            .input_ctx
            .streams()
            .find(|stream| stream.index() == audio_index)
        {
            let audio_tb = audio_stream.time_base();
            let audio_codec = audio_stream.parameters().id();
            let audio_details = codec::context::Context::from_parameters(audio_stream.parameters())
                .ok()
                .and_then(|ctx| ctx.decoder().audio().ok());
            if let Some(audio_decoder) = audio_details {
                let channels = audio_decoder.channels();
                let sample_rate = audio_decoder.rate();
                let sample_fmt = audio_decoder.format();
                let channel_layout = if audio_decoder.channel_layout().is_empty() {
                    format!("{}ch", channels)
                } else {
                    format!("{:?}", audio_decoder.channel_layout())
                };
                emit_debug(
                    app,
                    "audio_format",
                    format!(
                        "codec={:?} sample_rate={}Hz channels={} layout={} sample_fmt={:?} tb={}/{}",
                        audio_codec,
                        sample_rate,
                        channels,
                        channel_layout,
                        sample_fmt,
                        audio_tb.numerator(),
                        audio_tb.denominator()
                    ),
                );
            } else {
                emit_debug(
                    app,
                    "audio_format",
                    format!(
                        "codec={:?} tb={}/{}",
                        audio_codec,
                        audio_tb.numerator(),
                        audio_tb.denominator()
                    ),
                );
            }
        }
    }
    let audio_pipeline = build_audio_pipeline(
        &video_ctx.input_ctx,
        audio_stream_index,
        audio_controls,
        timing_controls,
    )?;
    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            width: video_ctx.output_width,
            height: video_ctx.output_height,
            fps: video_ctx.fps_value,
            duration_seconds: video_ctx.duration_seconds,
        },
    );
    renderer.reset_timeline(0.0, timing_controls.playback_rate() as f64);
    emit_debug(app, "running", "decode loop running");
    Ok(DecodeRuntime {
        loop_state: DecodeLoopState::new(video_ctx.fps_value, timing_controls.clone()),
        video_ctx,
        scaler: None,
        audio_pipeline,
        should_tail_eof,
    })
}

fn run_decode_loop(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    loop {
        if !app
            .state::<MediaState>()
            .stream
            .is_generation_current(stream_generation)
        {
            emit_debug(
                app,
                "stop",
                "stale decode generation observed; exiting decode loop",
            );
            return Ok(());
        }
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed; exiting decode loop");
            return Ok(());
        }

        // Apply speed changes immediately. Otherwise, rodio may keep playing already-queued samples
        // at the old speed until the next PCM append, which is especially noticeable when slowing down.
        if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
            let next_rate = clamp_playback_rate(timing_controls.playback_rate());
            if (next_rate - runtime.loop_state.last_applied_audio_rate).abs() > 1e-3 {
                runtime.loop_state.begin_rate_switch_settle();
                if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
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
                if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD
                    && next_rate < 1.0
                {
                    audio_state.output.player.clear();
                    audio_state.output.player.play();
                    runtime.loop_state.audio_clock = None;
                    runtime.loop_state.audio_queue_depth_sources = None;
                }
                runtime.loop_state.last_applied_audio_rate = next_rate;
            }
        }

        // Keep decode near real-time. If we run far ahead (especially with a tiny video queue),
        // we can reach EOF quickly and the video will appear to "end" while audio is still draining.
        let in_rate_switch_settle = runtime.loop_state.in_rate_switch_settle();
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
        let audio_now_seconds = runtime
            .loop_state
            .audio_clock
            .as_ref()
            .map(|clock| clock.now_seconds());
        if let (Some(video_pts), Some(audio_pts)) =
            (runtime.loop_state.last_video_pts_seconds, audio_now_seconds)
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
                &mut runtime.video_ctx.input_ctx,
                &mut runtime.video_ctx.decoder,
                target_seconds,
                &mut runtime.loop_state.playback_clock,
                &mut runtime.loop_state.current_position_seconds,
                runtime.audio_pipeline.as_mut(),
            )?;
            renderer.reset_timeline(
                target_seconds.max(0.0),
                timing_controls.playback_rate() as f64,
            );
            // Seek creates a discontinuity. Drop previous audio clock/queue observations
            // so video pacing does not use stale pre-seek timing and cause stutter.
            runtime.loop_state.audio_clock = None;
            runtime.loop_state.audio_queue_depth_sources = None;
            runtime.loop_state.active_seek_target_seconds = Some(target_seconds.max(0.0));
            runtime.loop_state.last_video_pts_seconds = None;
            runtime.loop_state.last_progress_emit = Instant::now() - Duration::from_millis(250);
            continue;
        }
        let mut packet = Packet::empty();
        match packet.read(&mut runtime.video_ctx.input_ctx) {
            Ok(_) => {
                // Approximate network read throughput from demuxed packet sizes.
                // This is best-effort and primarily used for UI feedback on URL playback.
                runtime.loop_state.update_network_window(packet.size());
                let audio_stream_index = runtime.audio_pipeline.as_ref().map(|audio| audio.stream_index);
                if packet.stream() == runtime.video_ctx.video_stream_index
                    || audio_stream_index.is_some_and(|index| packet.stream() == index)
                {
                    runtime
                        .loop_state
                        .update_media_required_window(packet.size());
                }
                let recording_target = current_recording_target(app, source)?;
                match (
                    runtime.loop_state.cache_writer.as_ref(),
                    recording_target.as_ref(),
                ) {
                    (None, Some(target)) => {
                        match CacheRemuxWriter::new(&runtime.video_ctx.input_ctx, target) {
                            Ok(writer) => {
                                emit_debug(
                                    app,
                                    "cache_recording",
                                    format!("start remux recording: {target}"),
                                );
                                runtime.loop_state.cache_writer = Some(writer);
                            }
                            Err(err) => {
                                update_cache_session_error(app, source, err.clone());
                                emit_debug(app, "cache_recording_error", err);
                            }
                        }
                    }
                    (Some(writer), Some(target)) if writer.output_path != *target => {
                        if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
                            writer.finish();
                        }
                        runtime.loop_state.cache_writer = None;
                    }
                    (Some(_), None) => {
                        if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
                            writer.finish();
                        }
                        runtime.loop_state.cache_writer = None;
                    }
                    _ => {}
                }
                if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
                    if let Err(err) = writer.write_packet(&runtime.video_ctx.input_ctx, &packet) {
                        writer.finish();
                        runtime.loop_state.cache_writer = None;
                        update_cache_session_error(app, source, err.clone());
                        emit_debug(app, "cache_recording_error", err);
                    }
                }
                if packet.stream() == runtime.video_ctx.video_stream_index {
                    runtime.loop_state.record_video_packet(
                        app,
                        &packet,
                        runtime.video_ctx.video_time_base,
                    );
                    if let Err(err) = runtime.video_ctx.decoder.send_packet(&packet) {
                        emit_debug(
                            app,
                            "decode_error_detail",
                            format!("video_send_packet_failed err={err}"),
                        );
                        return Err(format!("send packet failed: {err}"));
                    }
                    let mut drain_ctx = DrainFramesContext {
                        app,
                        renderer,
                        decoder: &mut runtime.video_ctx.decoder,
                        video_time_base: runtime.video_ctx.video_time_base,
                        scaler: &mut runtime.scaler,
                        duration_seconds: runtime.video_ctx.duration_seconds,
                        output_width: runtime.video_ctx.output_width,
                        output_height: runtime.video_ctx.output_height,
                        stop_flag,
                        playback_clock: &mut runtime.loop_state.playback_clock,
                        last_progress_emit: &mut runtime.loop_state.last_progress_emit,
                        current_position_seconds: &mut runtime.loop_state.current_position_seconds,
                        audio_clock: runtime.loop_state.audio_clock,
                        audio_queue_depth_sources: runtime.loop_state.audio_queue_depth_sources,
                        active_seek_target_seconds: &mut runtime
                            .loop_state
                            .active_seek_target_seconds,
                        last_video_pts_seconds: &mut runtime.loop_state.last_video_pts_seconds,
                        fps_window: &mut runtime.loop_state.fps_window,
                        frame_pipeline: &mut runtime.loop_state.frame_pipeline,
                        process_metrics: &mut runtime.loop_state.process_metrics,
                        audio_allowed_lead_seconds,
                        network_read_bps: runtime.loop_state.net_read_bps,
                        media_required_bps: runtime.loop_state.media_required_bps,
                        video_ts_window_start: &mut runtime.loop_state.video_ts_window_start,
                        video_ts_samples: &mut runtime.loop_state.video_ts_samples,
                        video_pts_missing: &mut runtime.loop_state.video_pts_missing,
                        video_pts_backtrack: &mut runtime.loop_state.video_pts_backtrack,
                        video_pts_jitter_abs_sum_ms: &mut runtime
                            .loop_state
                            .video_pts_jitter_abs_sum_ms,
                        video_pts_jitter_max_ms: &mut runtime.loop_state.video_pts_jitter_max_ms,
                        video_frame_type_window_start: &mut runtime
                            .loop_state
                            .video_frame_type_window_start,
                        video_frame_type_i: &mut runtime.loop_state.video_frame_type_i,
                        video_frame_type_p: &mut runtime.loop_state.video_frame_type_p,
                        video_frame_type_b: &mut runtime.loop_state.video_frame_type_b,
                        video_frame_type_other: &mut runtime.loop_state.video_frame_type_other,
                        stream_generation,
                    };
                    drain_frames(&mut drain_ctx)?;
                    continue;
                }
                if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
                    if packet.stream() == audio_state.stream_index {
                        if let Err(err) = audio_state.decoder.send_packet(&packet) {
                            emit_debug(
                                app,
                                "decode_error_detail",
                                format!("audio_send_packet_failed err={err}"),
                            );
                            return Err(format!("send audio packet failed: {err}"));
                        }
                        drain_audio_frames(
                            app,
                            audio_state,
                            stop_flag,
                            timing_controls,
                            &mut runtime.loop_state.audio_clock,
                            &mut runtime.loop_state.audio_queue_depth_sources,
                            &mut runtime.loop_state.active_seek_target_seconds,
                        )?;
                    }
                    continue;
                }
            }
            Err(FfmpegError::Eof) => {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                if runtime.should_tail_eof {
                    // For live sources (m3u8) and the growing cache-recording mp4, EOF is often
                    // temporary (no new data yet). Wait a bit and try reading again.
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
                break;
            }
            Err(_) => {
                if runtime.should_tail_eof {
                    std::thread::sleep(Duration::from_millis(50));
                }
                continue;
            }
        }
    }
    Ok(())
}

fn finish_decode_runtime(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
) -> Result<(), String> {
    if let Some(writer) = runtime.loop_state.cache_writer.as_mut() {
        writer.finish();
    }
    runtime
        .video_ctx
        .decoder
        .send_eof()
        .map_err(|err| format!("send eof failed: {err}"))?;
    let mut drain_ctx = DrainFramesContext {
        app,
        renderer,
        decoder: &mut runtime.video_ctx.decoder,
        video_time_base: runtime.video_ctx.video_time_base,
        scaler: &mut runtime.scaler,
        duration_seconds: runtime.video_ctx.duration_seconds,
        output_width: runtime.video_ctx.output_width,
        output_height: runtime.video_ctx.output_height,
        stop_flag,
        playback_clock: &mut runtime.loop_state.playback_clock,
        last_progress_emit: &mut runtime.loop_state.last_progress_emit,
        current_position_seconds: &mut runtime.loop_state.current_position_seconds,
        audio_clock: runtime.loop_state.audio_clock,
        audio_queue_depth_sources: runtime.loop_state.audio_queue_depth_sources,
        active_seek_target_seconds: &mut runtime.loop_state.active_seek_target_seconds,
        last_video_pts_seconds: &mut runtime.loop_state.last_video_pts_seconds,
        fps_window: &mut runtime.loop_state.fps_window,
        frame_pipeline: &mut runtime.loop_state.frame_pipeline,
        process_metrics: &mut runtime.loop_state.process_metrics,
        audio_allowed_lead_seconds: AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT,
        network_read_bps: runtime.loop_state.net_read_bps,
        media_required_bps: runtime.loop_state.media_required_bps,
        video_ts_window_start: &mut runtime.loop_state.video_ts_window_start,
        video_ts_samples: &mut runtime.loop_state.video_ts_samples,
        video_pts_missing: &mut runtime.loop_state.video_pts_missing,
        video_pts_backtrack: &mut runtime.loop_state.video_pts_backtrack,
        video_pts_jitter_abs_sum_ms: &mut runtime.loop_state.video_pts_jitter_abs_sum_ms,
        video_pts_jitter_max_ms: &mut runtime.loop_state.video_pts_jitter_max_ms,
        video_frame_type_window_start: &mut runtime.loop_state.video_frame_type_window_start,
        video_frame_type_i: &mut runtime.loop_state.video_frame_type_i,
        video_frame_type_p: &mut runtime.loop_state.video_frame_type_p,
        video_frame_type_b: &mut runtime.loop_state.video_frame_type_b,
        video_frame_type_other: &mut runtime.loop_state.video_frame_type_other,
        stream_generation,
    };
    drain_frames(&mut drain_ctx)?;
    if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
        audio_state
            .decoder
            .send_eof()
            .map_err(|err| format!("send audio eof failed: {err}"))?;
        drain_audio_frames(
            app,
            audio_state,
            stop_flag,
            timing_controls,
            &mut runtime.loop_state.audio_clock,
            &mut runtime.loop_state.audio_queue_depth_sources,
            &mut runtime.loop_state.active_seek_target_seconds,
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
    let mut tail_position_seconds = runtime.loop_state.current_position_seconds.max(0.0);
    let mut last_tail_tick = Instant::now();
    let mut last_tail_progress_emit = Instant::now() - Duration::from_millis(250);
    loop {
        if stop_flag.load(Ordering::Relaxed) {
            emit_debug(app, "stop", "stop flag observed during eof tail; exiting");
            return Ok(());
        }

        if let Some(audio_state) = runtime.audio_pipeline.as_mut() {
            let next_rate = clamp_playback_rate(timing_controls.playback_rate());
            if (next_rate - runtime.loop_state.last_applied_audio_rate).abs() > 1e-3 {
                if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
                    // Keep EOF tail pacing in sync with runtime rate changes.
                    clock.rebase_rate(next_rate as f64);
                }
                audio_state.output.player.set_speed(next_rate);
                let queued_sources = audio_state.output.player.len();
                if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD
                    && next_rate < 1.0
                {
                    audio_state.output.player.clear();
                    audio_state.output.player.play();
                }
                runtime.loop_state.last_applied_audio_rate = next_rate;
            }
        }

        // If duration is unknown, fall back to finishing when audio drains (if any).
        let duration_seconds = runtime.video_ctx.duration_seconds.max(0.0);
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

        let audio_done = runtime
            .audio_pipeline
            .as_ref()
            .map(|audio| audio.output.player.len() == 0)
            .unwrap_or(true);
        let duration_done =
            duration_seconds <= 0.0 || tail_position_seconds + 1e-3 >= duration_seconds;
        if audio_done && duration_done {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    update_playback_progress(
        app,
        tail_position_seconds,
        runtime.video_ctx.duration_seconds,
        true,
    )?;
    Ok(())
}
