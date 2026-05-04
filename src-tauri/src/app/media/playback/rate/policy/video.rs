use super::audio_queue::audio_queue_prefill_target;
use super::constants::{
    AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_LOCAL, AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_NETWORK,
    AUDIO_RATE_SWITCH_COVER_SECONDS_REALTIME, AUDIO_RATE_SWITCH_COVER_SECONDS_VIDEO,
    AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_LOCAL, AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_NETWORK,
    AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_REALTIME, AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_VIDEO,
    VIDEO_DRAIN_BATCH_LIMIT_CRITICAL, VIDEO_DRAIN_BATCH_LIMIT_LOW, VIDEO_DRAIN_BATCH_LIMIT_WARMUP,
};
use super::super::value::PlaybackRate;

pub fn audio_rate_switch_cover_seconds(
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> f64 {
    if is_realtime_source {
        return AUDIO_RATE_SWITCH_COVER_SECONDS_REALTIME;
    }
    if has_video_stream {
        return AUDIO_RATE_SWITCH_COVER_SECONDS_VIDEO;
    }
    if is_network_source {
        AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_NETWORK
    } else {
        AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_LOCAL
    }
}

pub fn audio_rate_switch_min_apply_seconds(
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> f64 {
    if is_realtime_source {
        return AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_REALTIME;
    }
    if has_video_stream {
        return AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_VIDEO;
    }
    if is_network_source {
        AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_NETWORK
    } else {
        AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_LOCAL
    }
}

pub fn video_drain_batch_limit(
    playback_rate: PlaybackRate,
    has_audio_stream: bool,
    audio_queue_depth: Option<usize>,
    is_realtime_source: bool,
    allow_heavy_video_burst: bool,
) -> Option<usize> {
    if !has_audio_stream {
        return None;
    }
    let prefill_target = audio_queue_prefill_target(playback_rate, true, is_realtime_source, false);
    let critical_threshold = (prefill_target / 2).max(1);
    match audio_queue_depth {
        None if is_realtime_source => Some(VIDEO_DRAIN_BATCH_LIMIT_CRITICAL),
        None => Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP),
        Some(depth) if depth <= critical_threshold => Some(VIDEO_DRAIN_BATCH_LIMIT_CRITICAL),
        Some(depth) if is_realtime_source && depth <= prefill_target.saturating_add(1) => {
            Some(VIDEO_DRAIN_BATCH_LIMIT_LOW)
        }
        Some(_) if is_realtime_source && playback_rate.as_f32() > 1.0 => {
            Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP)
        }
        Some(depth)
            if playback_rate.as_f32() >= 1.25 && depth <= prefill_target.saturating_add(1) =>
        {
            Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP)
        }
        Some(depth) if depth < prefill_target => Some(VIDEO_DRAIN_BATCH_LIMIT_LOW),
        Some(depth) if allow_heavy_video_burst && depth >= prefill_target.saturating_add(1) => None,
        Some(_) => None,
    }
}

