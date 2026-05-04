use super::demux_topup;
use super::{handle_video_soft_error, is_decoder_backpressure, DecodeRuntime, StopFlag, TimingControlsHandle};
use crate::app::media::playback::render::renderer::RendererState;
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::time::Duration;
use tauri::AppHandle;

use super::super::pacing::current_audio_allowed_lead_seconds;
use super::super::emit_debug;

pub(super) fn handle_video_packet(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &StopFlag,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    let Some(video_time_base) = runtime.video_ctx.video_time_base else {
        return Ok(());
    };
    runtime
        .loop_state
        .record_video_packet(app, packet, video_time_base);
    if packet.is_key() && !runtime.is_realtime_source {
        runtime
            .loop_state
            .begin_video_queue_boost(Duration::from_millis(super::VIDEO_QUEUE_BOOST_WINDOW_MS));
        renderer.boost_queue_capacity(
            super::RENDERER_QUEUE_BOOST_MIN_CAPACITY,
            Duration::from_millis(super::RENDERER_QUEUE_BOOST_WINDOW_MS),
        );
    }
    send_video_packet_with_backpressure_recovery(
        app,
        renderer,
        stop_flag,
        runtime,
        stream_generation,
        packet,
    )?;
    super::drain_video_frames(
        app,
        renderer,
        stop_flag,
        runtime,
        current_audio_allowed_lead_seconds(runtime),
        false,
        stream_generation,
    )
    .map_err(|err| handle_video_soft_error(app, runtime, "video_drain_failed", err))?;
    demux_topup::top_up_demux_audio_after_video(
        app,
        renderer,
        source,
        stop_flag,
        timing_controls,
        runtime,
        stream_generation,
    )?;
    Ok(())
}

fn send_video_packet_with_backpressure_recovery(
    app: &AppHandle,
    renderer: &RendererState,
    stop_flag: &StopFlag,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: &Packet,
) -> Result<(), String> {
    let mut attempts = 0usize;
    loop {
        match send_video_packet_once(runtime, packet) {
            Ok(()) => return Ok(()),
            Err(err)
                if is_decoder_backpressure(&err)
                    && attempts < super::VIDEO_SEND_PACKET_RETRY_LIMIT =>
            {
                attempts = attempts.saturating_add(1);
                if attempts == 1 {
                    emit_debug(
                        app,
                        "video_decoder_backpressure",
                        format!(
                            "send_packet would block; draining decoded frames before retry #{attempts}"
                        ),
                    );
                }
                super::drain_video_frames(
                    app,
                    renderer,
                    stop_flag,
                    runtime,
                    current_audio_allowed_lead_seconds(runtime),
                    true,
                    stream_generation,
                )
                .map_err(|drain_err| {
                    handle_video_soft_error(app, runtime, "video_drain_failed", drain_err)
                })?;
            }
            Err(err) => {
                return Err(handle_video_soft_error(
                    app,
                    runtime,
                    "video_send_packet_failed",
                    err.to_string(),
                ));
            }
        }
    }
}

fn send_video_packet_once(runtime: &mut DecodeRuntime, packet: &Packet) -> Result<(), FfmpegError> {
    let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
        return Ok(());
    };
    decoder.send_packet(packet)
}

