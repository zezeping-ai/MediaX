use crate::app::media::playback::rate::PlaybackRate;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::emit_debug;
use ffmpeg_next::frame;
use tauri::AppHandle;

const AUDIO_OUTPUT_LATENCY_COMPENSATION_MAX_SECONDS: f64 = 0.065;
const AUDIO_OUTPUT_LATENCY_COMPENSATION_SHALLOW_QUEUE_BONUS_SECONDS: f64 = 0.010;
const AUDIO_OUTPUT_LATENCY_COMPENSATION_SEEK_SETTLE_BONUS_SECONDS: f64 = 0.008;
const AUDIO_OUTPUT_LATENCY_COMPENSATION_WARMUP_MAX_SECONDS: f64 = 0.016;
const AUDIO_OUTPUT_LATENCY_COMPENSATION_TOTAL_MAX_SECONDS: f64 = 0.090;

pub(super) fn output_latency_compensation_seconds(
    output_wall_seconds: f64,
    queued_block_depth: usize,
    video_frame_duration_seconds: Option<f64>,
    in_seek_settle: bool,
    audio_sync_warmup_factor: f64,
) -> f64 {
    let latency_block_multiplier = if queued_block_depth <= 5 {
        3.0
    } else if queued_block_depth <= 7 {
        2.5
    } else {
        2.0
    };
    // With shallow software queues, the remaining device/mixer path becomes a larger fraction
    // of total audible latency. Add a small bounded reserve in that regime so the audio head
    // estimate does not run systematically ahead of what users actually hear.
    let shallow_queue_bonus_seconds = if queued_block_depth <= 5 {
        AUDIO_OUTPUT_LATENCY_COMPENSATION_SHALLOW_QUEUE_BONUS_SECONDS
    } else {
        0.0
    };
    let seek_settle_bonus_seconds = if in_seek_settle {
        AUDIO_OUTPUT_LATENCY_COMPENSATION_SEEK_SETTLE_BONUS_SECONDS
    } else {
        0.0
    };
    // Warmup compensation is kept outside the steady-state cap so startup / post-seek audio
    // head estimation can be slightly more conservative without permanently biasing sync.
    let warmup_bonus_seconds = audio_sync_warmup_factor.clamp(0.0, 1.0)
        * AUDIO_OUTPUT_LATENCY_COMPENSATION_WARMUP_MAX_SECONDS;
    let base_output_latency_seconds = (output_wall_seconds.max(0.0) * latency_block_multiplier
        + shallow_queue_bonus_seconds)
        .min(AUDIO_OUTPUT_LATENCY_COMPENSATION_MAX_SECONDS);
    let extra_output_latency_seconds = (base_output_latency_seconds
        + seek_settle_bonus_seconds
        + warmup_bonus_seconds)
        .min(AUDIO_OUTPUT_LATENCY_COMPENSATION_TOTAL_MAX_SECONDS);
    let display_hold_compensation_seconds = video_frame_duration_seconds
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value * 0.5)
        .unwrap_or(0.0);
    extra_output_latency_seconds + display_hold_compensation_seconds
}

pub(super) fn sync_audio_clock(
    decoded: &frame::Audio,
    time_base: ffmpeg_next::Rational,
    playback_rate: PlaybackRate,
    audio_clock: &mut Option<AudioClock>,
    active_seek_target_seconds: &mut Option<f64>,
) {
    if let Some(seconds) = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), time_base)
        .filter(|value| value.is_finite() && *value >= 0.0)
    {
        if let Some(target) = *active_seek_target_seconds {
            if seconds + 0.03 < target {
                return;
            }
            *active_seek_target_seconds = None;
        }
        if audio_clock.is_none() {
            *audio_clock = Some(AudioClock::new(
                seconds,
                playback_rate.as_f64(),
            ));
        }
    }
}

pub(super) fn sync_audio_clock_to_output_head(
    frame_start_seconds: Option<f64>,
    output_samples: usize,
    channels: usize,
    sample_rate: u32,
    playback_rate: PlaybackRate,
    queued_wall_seconds: f64,
    queued_block_depth: usize,
    video_frame_duration_seconds: Option<f64>,
    in_seek_settle: bool,
    audio_sync_warmup_factor: f64,
    tracked_playback_head_seconds: Option<f64>,
    audio_clock: &mut Option<AudioClock>,
) {
    if let Some(playback_head_seconds) =
        tracked_playback_head_seconds.filter(|value| value.is_finite() && *value >= 0.0)
    {
        let playback_rate_f64 = playback_rate.as_f64().max(0.25);
        match audio_clock.as_mut() {
            Some(clock) => clock.rebase_position(playback_head_seconds, playback_rate_f64),
            None => *audio_clock = Some(AudioClock::new(playback_head_seconds, playback_rate_f64)),
        }
        return;
    }
    let Some(frame_start_seconds) =
        frame_start_seconds.filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return;
    };
    if channels == 0 || sample_rate == 0 || output_samples == 0 {
        return;
    }
    let output_frames = output_samples / channels;
    if output_frames == 0 {
        return;
    }
    let playback_rate_f64 = playback_rate.as_f64().max(0.25);
    let output_wall_seconds = output_frames as f64 / sample_rate as f64;
    let enqueued_media_end_seconds = frame_start_seconds + output_wall_seconds * playback_rate_f64;
    let effective_queued_wall_seconds = queued_wall_seconds.max(0.0)
        + output_latency_compensation_seconds(
            output_wall_seconds,
            queued_block_depth,
            video_frame_duration_seconds,
            in_seek_settle,
            audio_sync_warmup_factor,
        );
    let playback_head_seconds =
        (enqueued_media_end_seconds - effective_queued_wall_seconds * playback_rate_f64).max(0.0);
    match audio_clock.as_mut() {
        Some(clock) => clock.rebase_position(playback_head_seconds, playback_rate_f64),
        None => *audio_clock = Some(AudioClock::new(playback_head_seconds, playback_rate_f64)),
    }
}

pub(super) fn should_drop_pre_seek_audio_frame(
    app: &AppHandle,
    decoded: &frame::Audio,
    time_base: ffmpeg_next::Rational,
    active_seek_target_seconds: &Option<f64>,
) -> bool {
    let Some(target) = *active_seek_target_seconds else {
        return false;
    };
    let Some(seconds) = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), time_base)
        .filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return false;
    };
    if seconds + 0.03 < target {
        emit_debug(
            app,
            "audio_seek_drop",
            format!(
                "drop stale audio frame pts={seconds:.3}s target={target:.3}s delta_ms={:.3}",
                (target - seconds) * 1000.0
            ),
        );
        return true;
    }
    false
}
