mod diagnostics;
mod sync;

use super::types::AudioPipeline;
use crate::app::media::playback::rate::{
    PlaybackRate, audio_queue_prefill_target, audio_queue_seconds_limit, output_staging_frames,
    rate_switch_cover_output_staging_frames, seek_refill_output_staging_frames,
    seek_settle_output_staging_frames,
};
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::audio::effective_playback_rate;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::emit_debug;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::frame;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

use self::diagnostics::emit_audio_debug_if_needed;
use self::sync::{
    should_drop_pre_seek_audio_frame, sync_audio_clock, sync_audio_clock_to_output_head,
};

pub(crate) fn drain_audio_frames(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    stop_flag: &Arc<AtomicBool>,
    applied_playback_rate: PlaybackRate,
    audio_clock: &mut Option<AudioClock>,
    audio_queue_depth_sources: &mut Option<usize>,
    audio_queued_seconds: &mut Option<f64>,
    active_seek_target_seconds: &mut Option<f64>,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    building_rate_switch_cover: bool,
    seeking_low_latency_refill: bool,
    in_seek_settle: bool,
    decoder_relief_mode: bool,
    force_low_latency_output: bool,
) -> Result<(), String> {
    audio_state.stats.packets = audio_state.stats.packets.saturating_add(1);
    let mut decoded = frame::Audio::empty();
    loop {
        let playback_rate = effective_playback_rate(applied_playback_rate, is_realtime_source);
        let min_prefill_queue_depth = audio_queue_prefill_target(
            playback_rate,
            has_video_stream,
            is_realtime_source,
            is_network_source,
        );
        if !decoder_relief_mode
            && should_yield_audio_decode(
            audio_state.output.queue_depth(),
            audio_state.output.queued_duration_seconds(),
            min_prefill_queue_depth,
            playback_rate,
            has_video_stream,
            is_realtime_source,
            is_network_source,
            building_rate_switch_cover,
            seeking_low_latency_refill,
            force_low_latency_output,
        )
        {
            break;
        }
        if audio_state.decoder.receive_frame(&mut decoded).is_err() {
            break;
        }
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
        let mut pcm = std::mem::take(&mut audio_state.scratch_pcm);
        audio_state
            .time_stretch
            .process_frame_into(&mut converted, playback_rate.as_f32(), &mut pcm)?;
        if pcm.is_empty() {
            audio_state.scratch_pcm = pcm;
            if !playback_rate.is_neutral() {
                let queue_depth = audio_state.output.queue_depth();
                let should_emit_pending = queue_depth <= 2
                    && audio_state
                        .stats
                        .last_time_stretch_pending_instant
                        .map(|last| {
                            Instant::now().saturating_duration_since(last)
                                >= Duration::from_millis(500)
                        })
                        .unwrap_or(true);
                if should_emit_pending {
                    audio_state.stats.last_time_stretch_pending_instant = Some(Instant::now());
                    emit_debug(
                        app,
                        "audio_time_stretch_pending",
                        format!(
                            "rate={:.2} decoded_pts={:?} converted_pts={:?} samples_per_ch={} queue_sources={} queue_seconds={:.3}",
                            playback_rate.as_f32(),
                            decoded.pts().or(decoded.timestamp()),
                            converted.pts(),
                            samples_per_channel,
                            queue_depth,
                            audio_state.output.queued_duration_seconds(),
                        ),
                    );
                }
            }
            continue;
        }
        if audio_state.output.queue_depth() == 0 {
            if audio_state.stats.intentional_refill_pending {
                if !audio_state.stats.intentional_refill_logged {
                    emit_debug(
                        app,
                        "audio_refill",
                        format!(
                            "planned queue refill after discontinuity rate={:.2}",
                            playback_rate.as_f32(),
                        ),
                    );
                    audio_state.stats.intentional_refill_logged = true;
                }
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
            audio_state.scratch_pcm = pcm;
            continue;
        }
        sync_audio_clock(
            &decoded,
            audio_state.time_base,
            playback_rate,
            audio_clock,
            active_seek_target_seconds,
        );

        audio_state.apply_discontinuity_smoothing(&mut pcm, converted.channels());
        let staging_frames = if seeking_low_latency_refill {
            seek_refill_output_staging_frames(has_video_stream)
        } else if decoder_relief_mode {
            rate_switch_cover_output_staging_frames(has_video_stream, is_realtime_source)
        } else if in_seek_settle {
            seek_settle_output_staging_frames(has_video_stream, is_network_source)
        } else if building_rate_switch_cover {
            rate_switch_cover_output_staging_frames(has_video_stream, is_realtime_source)
        } else {
            output_staging_frames(playback_rate, has_video_stream, is_network_source)
        };
        let force_flush_partial = (force_low_latency_output && !decoder_relief_mode)
            || seeking_low_latency_refill
            || (!building_rate_switch_cover
                && !decoder_relief_mode
                && audio_state.output.queue_depth() < min_prefill_queue_depth);
        let output_blocks = audio_state.stage_output_pcm(
            &pcm,
            converted.channels(),
            staging_frames,
            force_flush_partial,
        );
        let output_samples: usize = output_blocks.iter().map(Vec::len).sum();
        for block in output_blocks {
            audio_state.stats.queued_samples = audio_state
                .stats
                .queued_samples
                .saturating_add(block.len() as u64);
            audio_state
                .output
                .append_pcm_f32_owned(converted.rate(), converted.channels(), block);
        }
        sync_audio_clock_to_output_head(
            timestamp_to_seconds(converted.timestamp(), converted.pts(), audio_state.time_base),
            output_samples,
            converted.channels() as usize,
            converted.rate(),
            playback_rate,
            audio_state.output.queued_duration_seconds(),
            audio_clock,
        );
        if audio_state.output.queue_depth() > 0 {
            audio_state.mark_refill_completed();
            if seeking_low_latency_refill && !audio_state.stats.seek_refill_logged {
                emit_debug(
                    app,
                    "audio_seek_refill_ready",
                    format!(
                        "seek refill produced output queue_sources={} queue_seconds={:.3}",
                        audio_state.output.queue_depth(),
                        audio_state.output.queued_duration_seconds(),
                    ),
                );
                audio_state.stats.seek_refill_logged = true;
                *audio_queue_depth_sources = Some(audio_state.output.queue_depth());
                *audio_queued_seconds = Some(audio_state.output.queued_duration_seconds());
                pcm.clear();
                audio_state.scratch_pcm = pcm;
                break;
            }
        }
        *audio_queue_depth_sources = Some(audio_state.output.queue_depth());
        *audio_queued_seconds = Some(audio_state.output.queued_duration_seconds());

        if should_yield_audio_decode(
            audio_state.output.queue_depth(),
            audio_state.output.queued_duration_seconds(),
            min_prefill_queue_depth,
            playback_rate,
            has_video_stream,
            is_realtime_source,
            is_network_source,
            building_rate_switch_cover,
            seeking_low_latency_refill,
            force_low_latency_output,
        ) {
            pcm.clear();
            audio_state.scratch_pcm = pcm;
            break;
        }

        emit_audio_debug_if_needed(
            app,
            audio_state,
            playback_rate,
            channels,
            samples_per_channel,
            pcm.len(),
            audio_state.output.queued_duration_seconds(),
            audio_clock,
        );
        pcm.clear();
        audio_state.scratch_pcm = pcm;
    }
    Ok(())
}

fn should_yield_audio_decode(
    queue_depth: usize,
    queued_seconds: f64,
    min_prefill_queue_depth: usize,
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    building_rate_switch_cover: bool,
    seeking_low_latency_refill: bool,
    force_low_latency_output: bool,
) -> bool {
    if has_video_stream
        || is_realtime_source
        || building_rate_switch_cover
        || seeking_low_latency_refill
        || force_low_latency_output
    {
        return false;
    }
    if queue_depth < min_prefill_queue_depth {
        return false;
    }
    let Some(queue_seconds_limit) = audio_queue_seconds_limit(
        playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    ) else {
        return false;
    };
    queued_seconds + 1e-3 >= queue_seconds_limit
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
