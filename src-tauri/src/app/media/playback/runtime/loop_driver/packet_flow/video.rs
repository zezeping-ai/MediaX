use super::demux_topup;
use super::{
    handle_video_soft_error, is_decoder_backpressure, DecodeRuntime, StopFlag, TimingControlsHandle,
    VIDEO_SEND_PACKET_SPIN_LIMIT,
};
use crate::app::media::playback::render::renderer::RendererState;
use ffmpeg_next::Error as FfmpegError;
use ffmpeg_next::Packet;
use std::time::Duration;
use tauri::AppHandle;

use super::super::pacing::current_audio_allowed_lead_seconds;
use super::super::emit_debug;
use super::super::sleep_with_stop_flag;

pub(super) fn handle_video_packet(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stop_flag: &StopFlag,
    timing_controls: &TimingControlsHandle,
    runtime: &mut DecodeRuntime,
    stream_generation: u32,
    packet: Packet,
) -> Result<(), String> {
    let Some(video_time_base) = runtime.video_ctx.video_time_base else {
        return Ok(());
    };
    runtime
        .loop_state
        .record_video_packet(app, &packet, video_time_base);
    if packet.is_key() && !runtime.is_realtime_source {
        runtime
            .loop_state
            .begin_video_queue_boost(Duration::from_millis(super::VIDEO_QUEUE_BOOST_WINDOW_MS));
        renderer.boost_queue_capacity(
            super::RENDERER_QUEUE_BOOST_MIN_CAPACITY,
            Duration::from_millis(super::RENDERER_QUEUE_BOOST_WINDOW_MS),
        );
    }

    // Avoid blocking the decode loop on persistent decoder backpressure: after a small number
    // of drain+retry attempts, stash this muxed packet and retry in the next outer iteration.
    let mut attempts = 0usize;
    loop {
        match send_video_packet_once(runtime, &packet) {
            Ok(()) => break,
            Err(err)
                if is_decoder_backpressure(&err)
                    && attempts < super::VIDEO_SEND_PACKET_RETRY_LIMIT =>
            {
                attempts = attempts.saturating_add(1);
                if attempts == 1 {
                    emit_debug(
                        app,
                        "video_decoder_backpressure",
                        "send_packet would block; draining decoded frames before retry #1",
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
                std::thread::yield_now();
                if attempts >= 2 {
                    sleep_with_stop_flag(stop_flag, Duration::from_millis((attempts as u64).min(3)));
                }
                if attempts >= VIDEO_SEND_PACKET_SPIN_LIMIT {
                    runtime.demux_packet_stash = Some(packet);
                    return Ok(());
                }
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

fn send_video_packet_once(runtime: &mut DecodeRuntime, packet: &Packet) -> Result<(), FfmpegError> {
    let Some(decoder) = runtime.video_ctx.decoder.as_mut() else {
        return Ok(());
    };
    decoder.send_packet(packet)
}

