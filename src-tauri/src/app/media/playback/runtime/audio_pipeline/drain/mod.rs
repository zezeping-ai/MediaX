mod sync;

use super::types::AudioPipeline;
use crate::app::media::playback::rate::{
    audio_queue_prefill_target, audio_queue_refill_floor_seconds, output_staging_frames,
    rate_switch_cover_output_staging_frames, seek_refill_output_staging_frames,
    seek_settle_output_staging_frames, PlaybackRate,
};
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::audio::effective_playback_rate;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::sync_clock::SyncClockSample;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::frame;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;

use self::sync::{
    output_latency_compensation_seconds, should_drop_pre_seek_audio_frame, sync_audio_clock,
    sync_audio_clock_to_queue_estimate,
};

const AUDIO_OUTPUT_MEDIA_CURSOR_REBASE_THRESHOLD_SECONDS: f64 = 0.150;
const VIDEO_STEADY_OUTPUT_BLOCK_MIN_SECONDS: f64 = 0.028;
const VIDEO_STEADY_OUTPUT_BLOCK_MAX_SECONDS: f64 = 0.040;
const AUDIO_ONLY_STEADY_OUTPUT_BLOCK_SECONDS: f64 = 0.024;
const MIN_PARTIAL_FLUSH_FRAMES: usize = 1024;
const AUDIO_LOW_QUEUE_LOG_INTERVAL_PACKETS: u64 = 60;
const AUDIO_LOW_QUEUE_THRESHOLD_SECONDS: f64 = 0.050;

#[derive(Clone, Copy)]
pub(crate) struct AudioDrainParams {
    pub applied_playback_rate: PlaybackRate,
    pub has_video_stream: bool,
    pub is_realtime_source: bool,
    pub is_network_source: bool,
    pub building_rate_switch_cover: bool,
    pub seeking_low_latency_refill: bool,
    pub in_seek_settle: bool,
    pub audio_sync_warmup_factor: f64,
    pub decoder_relief_mode: bool,
    pub force_low_latency_output: bool,
    pub video_frame_duration_seconds: Option<f64>,
}

pub(crate) struct AudioDrainStateRefs<'a> {
    pub stop_flag: &'a Arc<AtomicBool>,
    pub audio_clock: &'a mut Option<AudioClock>,
    pub observed_audio_clock: &'a mut Option<SyncClockSample>,
    pub audio_queue_depth_sources: &'a mut Option<usize>,
    pub audio_queued_seconds: &'a mut Option<f64>,
    pub active_seek_target_seconds: &'a mut Option<f64>,
}

fn sample_observed_audio_clock(
    audio_state: &AudioPipeline,
    output_wall_seconds: f64,
    video_frame_duration_seconds: Option<f64>,
    in_seek_settle: bool,
    audio_sync_warmup_factor: f64,
) -> Option<SyncClockSample> {
    let estimated_extra_latency_seconds = output_latency_compensation_seconds(
        output_wall_seconds,
        audio_state.output.queue_depth(),
        video_frame_duration_seconds,
        in_seek_settle,
        audio_sync_warmup_factor,
    );
    audio_state
        .output
        .observed_playback_head_position(estimated_extra_latency_seconds)
        .and_then(SyncClockSample::from_audio_output_position)
}

fn steady_state_output_staging_frames(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_network_source: bool,
    output_sample_rate: u32,
    video_frame_duration_seconds: Option<f64>,
) -> usize {
    let policy_frames = output_staging_frames(playback_rate, has_video_stream, is_network_source);
    let target_block_seconds = if has_video_stream {
        video_frame_duration_seconds
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| {
                value.clamp(
                    VIDEO_STEADY_OUTPUT_BLOCK_MIN_SECONDS,
                    VIDEO_STEADY_OUTPUT_BLOCK_MAX_SECONDS,
                )
            })
            .unwrap_or(1.0 / 30.0)
    } else {
        AUDIO_ONLY_STEADY_OUTPUT_BLOCK_SECONDS
    };
    let target_frames = ((output_sample_rate.max(1) as f64) * target_block_seconds).round()
        as usize;
    policy_frames.max(target_frames.max(256))
}

pub(crate) fn drain_audio_frames(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    state_refs: AudioDrainStateRefs<'_>,
    params: AudioDrainParams,
) -> Result<(), String> {
    let AudioDrainStateRefs {
        stop_flag,
        audio_clock,
        observed_audio_clock,
        audio_queue_depth_sources,
        audio_queued_seconds,
        active_seek_target_seconds,
    } = state_refs;
    let AudioDrainParams {
        applied_playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
        building_rate_switch_cover,
        seeking_low_latency_refill,
        in_seek_settle,
        audio_sync_warmup_factor,
        decoder_relief_mode,
        force_low_latency_output,
        video_frame_duration_seconds,
    } = params;
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
            let queue_depth = audio_state.output.queue_depth();
            let queued_seconds = audio_state.output.queued_duration_seconds();
            if queue_depth <= 1 || queued_seconds <= AUDIO_LOW_QUEUE_THRESHOLD_SECONDS {
                if !audio_state.stats.decode_supply_gap_logged {
                    crate::app::media::playback::runtime::emit_debug(
                        app,
                        "audio_decode_supply_gap",
                        format!(
                            "receive_frame empty while queue is shallow queue_depth={} queued_ms={:.2} decoder_relief={} low_latency={} seek_refill={} rate_switch_cover={}",
                            queue_depth,
                            queued_seconds * 1000.0,
                            decoder_relief_mode,
                            force_low_latency_output,
                            seeking_low_latency_refill,
                            building_rate_switch_cover,
                        ),
                    );
                    audio_state.stats.decode_supply_gap_logged = true;
                }
            }
            break;
        }
        if audio_state.stats.decode_supply_gap_logged {
            crate::app::media::playback::runtime::emit_debug(
                app,
                "audio_decode_supply_recovered",
                format!(
                    "decode supply recovered queue_depth={} queued_ms={:.2}",
                    audio_state.output.queue_depth(),
                    audio_state.output.queued_duration_seconds() * 1000.0,
                ),
            );
            audio_state.stats.decode_supply_gap_logged = false;
        }
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        audio_state.stats.decoded_frames = audio_state.stats.decoded_frames.saturating_add(1);
        normalize_decoded_audio_frame(&mut decoded, &audio_state.decoder);
        let decoded_timestamp_seconds =
            timestamp_to_seconds(decoded.timestamp(), decoded.pts(), audio_state.time_base);
        ensure_resampler_matches_frame(app, audio_state, &decoded)?;
        let mut converted = frame::Audio::empty();
        audio_state
            .resampler
            .run(&decoded, &mut converted)
            .map_err(|err| format!("audio resample failed: {err}"))?;
        if converted.pts().is_none() {
            converted.set_pts(decoded.pts().or(decoded.timestamp()));
        }

        let mut pcm = std::mem::take(&mut audio_state.scratch_pcm);
        audio_state.time_stretch.process_frame_into(
            &mut converted,
            playback_rate.as_f32(),
            &mut pcm,
        )?;
        if pcm.is_empty() {
            audio_state.scratch_pcm = pcm;
            continue;
        }
        if audio_state.output.queue_depth() == 0 {
            if !audio_state.stats.intentional_refill_pending {
                audio_state.stats.underrun_count =
                    audio_state.stats.underrun_count.saturating_add(1);
                crate::app::media::playback::runtime::emit_debug(
                    app,
                    "audio_output_underrun",
                    format!(
                        "underrun detected count={} decoder_relief={} seek_refill={} rate_switch_cover={}",
                        audio_state.stats.underrun_count,
                        decoder_relief_mode,
                        seeking_low_latency_refill,
                        building_rate_switch_cover,
                    ),
                );
            }
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
            steady_state_output_staging_frames(
                playback_rate,
                has_video_stream,
                is_network_source,
                audio_state.output_sample_rate,
                video_frame_duration_seconds,
            )
        };
        let force_flush_partial = should_force_partial_output_flush(
            audio_state.output.queue_depth(),
            audio_state.output.queued_duration_seconds(),
            playback_rate,
            has_video_stream,
            is_realtime_source,
            is_network_source,
            building_rate_switch_cover,
            seeking_low_latency_refill,
            force_low_latency_output,
            decoder_relief_mode,
        );
        let min_partial_flush_frames = if has_video_stream {
            staging_frames.min(MIN_PARTIAL_FLUSH_FRAMES)
        } else {
            staging_frames.min(MIN_PARTIAL_FLUSH_FRAMES / 2)
        };
        let staged_output = audio_state.stage_output_pcm_owned(
            pcm,
            converted.channels(),
            staging_frames,
            force_flush_partial,
            min_partial_flush_frames,
        );
        let output_samples: usize = staged_output.blocks.iter().map(Vec::len).sum();
        let output_sample_rate = audio_state.output_sample_rate.max(1);
        let playback_rate_f64 = playback_rate.as_f64().max(0.25);
        let mut next_output_media_start_seconds =
            align_output_media_cursor(audio_state, decoded_timestamp_seconds);
        let first_output_media_start_seconds = next_output_media_start_seconds;
        let mut queued_samples_before_blocks = 0usize;
        for block in staged_output.blocks {
            let block_samples = block.len();
            let block_frames = block_samples / usize::from(converted.channels().max(1));
            let block_wall_seconds = block_frames as f64 / output_sample_rate as f64;
            let block_media_start_seconds = next_output_media_start_seconds;
            let block_media_duration_seconds = block_wall_seconds * playback_rate_f64;
            audio_state.stats.queued_samples = audio_state
                .stats
                .queued_samples
                .saturating_add(block_samples as u64);
            audio_state.output.append_pcm_f32_owned(
                output_sample_rate,
                converted.channels(),
                block,
                block_media_start_seconds,
                block_media_duration_seconds,
            );
            next_output_media_start_seconds = block_media_start_seconds
                .map(|start_seconds| (start_seconds + block_media_duration_seconds).max(0.0));
            queued_samples_before_blocks =
                queued_samples_before_blocks.saturating_add(block_samples);
        }
        if queued_samples_before_blocks > 0 {
            audio_state.output_media_cursor_seconds = next_output_media_start_seconds;
        }
        let output_frames = output_samples / usize::from(converted.channels().max(1));
        let output_wall_seconds = output_frames as f64 / output_sample_rate as f64;
        *observed_audio_clock = sample_observed_audio_clock(
            audio_state,
            output_wall_seconds,
            video_frame_duration_seconds,
            in_seek_settle,
            audio_sync_warmup_factor,
        );
        sync_audio_clock_to_queue_estimate(
            first_output_media_start_seconds,
            output_samples,
            converted.channels() as usize,
            output_sample_rate,
            playback_rate,
            audio_state.output.queued_duration_seconds(),
            audio_state.output.queue_depth(),
            video_frame_duration_seconds,
            in_seek_settle,
            audio_sync_warmup_factor,
            audio_clock,
        );
        if audio_state.output.queue_depth() > 0 {
            audio_state.mark_refill_completed();
            if audio_state.output.is_paused()
                && should_resume_audio_output(
                    audio_state.output.queue_depth(),
                    audio_state.output.queued_duration_seconds(),
                    min_prefill_queue_depth,
                    playback_rate,
                    has_video_stream,
                    is_realtime_source,
                    is_network_source,
                    seeking_low_latency_refill,
                    force_low_latency_output,
                )
            {
                crate::app::media::playback::runtime::emit_debug(
                    app,
                    "audio_output_resume",
                    format!(
                        "resume output queue_depth={} queued_ms={:.2}",
                        audio_state.output.queue_depth(),
                        audio_state.output.queued_duration_seconds() * 1000.0,
                    ),
                );
                eprintln!(
                    "audio resume debug: queue_depth={} queued_ms={:.2}",
                    audio_state.output.queue_depth(),
                    audio_state.output.queued_duration_seconds() * 1000.0,
                );
                audio_state.output.resume();
            }
            if seeking_low_latency_refill && !audio_state.stats.seek_refill_logged {
                audio_state.stats.seek_refill_logged = true;
                *audio_queue_depth_sources = Some(audio_state.output.queue_depth());
                *audio_queued_seconds = Some(audio_state.output.queued_duration_seconds());
                audio_state.scratch_pcm = staged_output.scratch;
                break;
            }
        }
        *audio_queue_depth_sources = Some(audio_state.output.queue_depth());
        *audio_queued_seconds = Some(audio_state.output.queued_duration_seconds());
        let queue_depth = audio_state.output.queue_depth();
        let queued_seconds = audio_state.output.queued_duration_seconds();
        if queue_depth <= 1 || queued_seconds <= AUDIO_LOW_QUEUE_THRESHOLD_SECONDS {
            if !audio_state.stats.low_queue_logged
                || audio_state
                    .stats
                    .low_queue_log_counter
                    .saturating_add(1)
                    % AUDIO_LOW_QUEUE_LOG_INTERVAL_PACKETS
                    == 0
            {
                crate::app::media::playback::runtime::emit_debug(
                    app,
                    "audio_queue_low",
                    format!(
                        "queue shallow queue_depth={} queued_ms={:.2} prefill_target={} decoder_relief={} low_latency={} seek_refill={} seek_settle={} rate_switch_cover={}",
                        queue_depth,
                        queued_seconds * 1000.0,
                        min_prefill_queue_depth,
                        decoder_relief_mode,
                        force_low_latency_output,
                        seeking_low_latency_refill,
                        in_seek_settle,
                        building_rate_switch_cover,
                    ),
                );
                audio_state.stats.low_queue_logged = true;
                audio_state.stats.low_queue_log_counter = 0;
            } else {
                audio_state.stats.low_queue_log_counter =
                    audio_state.stats.low_queue_log_counter.saturating_add(1);
            }
        } else {
            if audio_state.stats.low_queue_logged {
                crate::app::media::playback::runtime::emit_debug(
                    app,
                    "audio_queue_recovered",
                    format!(
                        "queue recovered queue_depth={} queued_ms={:.2} prefill_target={}",
                        queue_depth,
                        queued_seconds * 1000.0,
                        min_prefill_queue_depth,
                    ),
                );
            }
            audio_state.stats.low_queue_logged = false;
            audio_state.stats.low_queue_log_counter = 0;
        }

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
            audio_state.scratch_pcm = staged_output.scratch;
            break;
        }
        audio_state.scratch_pcm = staged_output.scratch;
    }
    Ok(())
}

fn align_output_media_cursor(
    audio_state: &mut AudioPipeline,
    decoded_timestamp_seconds: Option<f64>,
) -> Option<f64> {
    let decoded_timestamp_seconds =
        decoded_timestamp_seconds.filter(|value| value.is_finite() && *value >= 0.0);
    match (
        audio_state.output_media_cursor_seconds,
        decoded_timestamp_seconds,
    ) {
        (Some(cursor_seconds), Some(decoded_seconds))
            if (decoded_seconds - cursor_seconds).abs()
                > AUDIO_OUTPUT_MEDIA_CURSOR_REBASE_THRESHOLD_SECONDS =>
        {
            audio_state.output_media_cursor_seconds = Some(decoded_seconds);
        }
        (None, Some(decoded_seconds)) => {
            audio_state.output_media_cursor_seconds = Some(decoded_seconds);
        }
        _ => {}
    }
    audio_state.output_media_cursor_seconds
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
    if is_realtime_source
        || building_rate_switch_cover
        || seeking_low_latency_refill
        || force_low_latency_output
    {
        return false;
    }
    if queue_depth < min_prefill_queue_depth {
        return false;
    }
    let Some(refill_floor_seconds) = audio_queue_refill_floor_seconds(
        playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    ) else {
        return false;
    };
    queued_seconds + 1e-3 >= refill_floor_seconds
}

fn should_force_partial_output_flush(
    queue_depth: usize,
    queued_seconds: f64,
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    building_rate_switch_cover: bool,
    seeking_low_latency_refill: bool,
    force_low_latency_output: bool,
    decoder_relief_mode: bool,
) -> bool {
    if seeking_low_latency_refill {
        return true;
    }
    if force_low_latency_output && !decoder_relief_mode {
        return true;
    }
    if is_realtime_source || building_rate_switch_cover || decoder_relief_mode {
        return false;
    }
    // In steady A/V playback, aggressively flushing undersized PCM fragments keeps the CPAL
    // queue oscillating around tiny ~20ms blocks. Only do that when the queue is truly at risk
    // of starving, otherwise let the staging buffer coalesce into a healthier output block.
    if queue_depth == 0 {
        return true;
    }
    let emergency_floor_seconds = audio_queue_refill_floor_seconds(
        playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    )
    .map(|seconds| (seconds * 0.35).max(0.025))
    .unwrap_or(0.025);
    queued_seconds + 1e-3 < emergency_floor_seconds && queue_depth <= 1
}

fn should_resume_audio_output(
    queue_depth: usize,
    queued_seconds: f64,
    min_prefill_queue_depth: usize,
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    seeking_low_latency_refill: bool,
    force_low_latency_output: bool,
) -> bool {
    if queue_depth == 0 {
        return false;
    }
    if is_realtime_source || seeking_low_latency_refill || force_low_latency_output {
        return true;
    }
    let Some(refill_floor_seconds) = audio_queue_refill_floor_seconds(
        playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    ) else {
        return false;
    };
    let resume_floor_seconds = if has_video_stream {
        (refill_floor_seconds + 0.08).max(0.30)
    } else if is_network_source {
        (refill_floor_seconds + 0.04).max(0.16)
    } else {
        (refill_floor_seconds + 0.02).max(0.08)
    };
    queue_depth >= min_prefill_queue_depth && queued_seconds + 1e-3 >= resume_floor_seconds
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
    _app: &AppHandle,
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
    audio_state.resampler = ResamplingContext::get(
        frame_format,
        frame_layout,
        frame_rate,
        audio_state.output_sample_format.ffmpeg_sample_format(),
        frame_layout,
        audio_state.output_sample_rate.max(1),
    )
    .map_err(|err| {
        format!(
            "reconfigure audio resampler failed: in_fmt={frame_format:?} in_rate={frame_rate}Hz in_layout={frame_layout:?} out_fmt={} out_rate={}Hz err={err}",
            audio_state.output_sample_format.debug_label(),
            audio_state.output_sample_rate.max(1),
        )
    })?;
    Ok(())
}
