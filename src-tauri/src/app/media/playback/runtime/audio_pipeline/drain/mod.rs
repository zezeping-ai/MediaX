mod diagnostics;
mod sync;

use super::types::AudioPipeline;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::state::TimingControls;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::frame;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;

use self::diagnostics::emit_audio_debug_if_needed;
use self::sync::{should_drop_pre_seek_audio_frame, sync_audio_clock};

pub(crate) fn drain_audio_frames(
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
        normalize_decoded_audio_frame(&mut decoded, &audio_state.decoder);
        ensure_resampler_matches_frame(app, audio_state, &decoded)?;
        let mut converted = frame::Audio::empty();
        audio_state
            .resampler
            .run(&decoded, &mut converted)
            .map_err(|err| format!("audio resample failed: {err}"))?;
        if converted.pts().is_none() {
            converted.set_pts(decoded.pts().or(decoded.timestamp()));
        }

        let channels = converted.channels().max(1) as usize;
        let samples_per_channel = converted.samples();
        let mut pcm = audio_state.time_stretch.process_frame(
            &converted,
            clamp_playback_rate(timing_controls.playback_rate()),
        )?;
        if pcm.is_empty() {
            if (clamp_playback_rate(timing_controls.playback_rate()) - 1.0).abs() > 1e-3 {
                emit_debug(
                    app,
                    "audio_time_stretch_pending",
                    format!(
                        "rate={:.2} decoded_pts={:?} converted_pts={:?} samples_per_ch={}",
                        timing_controls.playback_rate(),
                        decoded.pts().or(decoded.timestamp()),
                        converted.pts(),
                        samples_per_channel,
                    ),
                );
            }
            continue;
        }
        if audio_state.output.queue_depth() == 0 {
            if audio_state.stats.intentional_refill_pending {
                emit_debug(
                    app,
                    "audio_refill",
                    format!(
                        "planned queue refill after discontinuity rate={:.2}",
                        timing_controls.playback_rate(),
                    ),
                );
            } else {
                audio_state.stats.underrun_count =
                    audio_state.stats.underrun_count.saturating_add(1);
            }
        }
        if audio_state.output.is_paused() {
            audio_state.output.resume();
            emit_debug(
                app,
                "audio_resume",
                "audio player resumed from paused state",
            );
        }
        if should_drop_pre_seek_audio_frame(
            app,
            &decoded,
            audio_state.time_base,
            active_seek_target_seconds,
        ) {
            continue;
        }
        sync_audio_clock(
            &decoded,
            audio_state.time_base,
            timing_controls,
            audio_clock,
            active_seek_target_seconds,
        );

        audio_state.apply_discontinuity_smoothing(&mut pcm, converted.channels());
        audio_state.mark_refill_completed();
        let force_flush_partial = audio_state.output.queue_depth() <= 1;
        let output_blocks =
            audio_state.stage_output_pcm(&pcm, converted.channels(), force_flush_partial);
        for block in output_blocks {
            audio_state.stats.queued_samples = audio_state
                .stats
                .queued_samples
                .saturating_add(block.len() as u64);
            audio_state
                .output
                .append_pcm_f32(converted.rate(), converted.channels(), &block);
        }
        *audio_queue_depth_sources = Some(audio_state.output.queue_depth());

        emit_audio_debug_if_needed(
            app,
            audio_state,
            timing_controls,
            channels,
            samples_per_channel,
            pcm.len(),
            audio_clock,
        );
    }
    Ok(())
}

fn normalize_decoded_audio_frame(frame: &mut frame::Audio, decoder: &ffmpeg_next::decoder::Audio) {
    if frame.channel_layout().is_empty() {
        let fallback_layout = if decoder.channel_layout().is_empty() {
            ChannelLayout::default(frame.channels().max(1).into())
        } else {
            decoder.channel_layout()
        };
        frame.set_channel_layout(fallback_layout);
    }
    if frame.rate() == 0 {
        frame.set_rate(decoder.rate());
    }
}

fn ensure_resampler_matches_frame(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    frame: &frame::Audio,
) -> Result<(), String> {
    let input = audio_state.resampler.input();
    let frame_layout = frame.channel_layout();
    let frame_rate = frame.rate();
    let frame_format = frame.format();
    if input.format == frame_format
        && input.channel_layout == frame_layout
        && input.rate == frame_rate
    {
        return Ok(());
    }

    emit_debug(
        app,
        "audio_resampler_reconfig",
        format!(
            "input changed from fmt={:?} rate={}Hz layout={:?} to fmt={:?} rate={}Hz layout={:?}",
            input.format, input.rate, input.channel_layout, frame_format, frame_rate, frame_layout,
        ),
    );
    audio_state.resampler = ResamplingContext::get(
        frame_format,
        frame_layout,
        frame_rate,
        audio_state.output_sample_format.ffmpeg_sample_format(),
        frame_layout,
        frame_rate,
    )
    .map_err(|err| {
        format!(
            "reconfigure audio resampler failed: in_fmt={frame_format:?} in_rate={frame_rate}Hz in_layout={frame_layout:?} out_fmt={} err={err}",
            audio_state.output_sample_format.debug_label()
        )
    })?;
    Ok(())
}
