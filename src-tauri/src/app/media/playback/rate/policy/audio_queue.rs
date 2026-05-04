use super::constants::{
    AUDIO_QUEUE_PREFILL_AUDIO_ONLY_LOCAL, AUDIO_QUEUE_PREFILL_AUDIO_ONLY_NETWORK, AUDIO_QUEUE_PREFILL_REALTIME_VIDEO,
    AUDIO_QUEUE_PREFILL_VIDEO, AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_DEFAULT,
    AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_FAST, AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_SLOW,
    AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_DEFAULT, AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_FAST, AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_SLOW,
    AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_LOCAL, AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_NETWORK,
    AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_LOCAL, AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_NETWORK,
    AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_VIDEO, BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT,
};
use super::super::value::PlaybackRate;

pub fn audio_queue_depth_limit(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> usize {
    if has_video_stream && is_realtime_source {
        return realtime_audio_queue_depth_limit(playback_rate);
    }
    let playback_rate = playback_rate.as_f32();
    if !has_video_stream {
        return if is_network_source {
            if playback_rate >= 1.5 {
                10
            } else if playback_rate >= 1.25 {
                12
            } else if playback_rate <= 0.75 {
                AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_NETWORK.saturating_add(4)
            } else {
                AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_NETWORK
            }
        } else if playback_rate >= 1.5 {
            6
        } else if playback_rate >= 1.25 {
            7
        } else if playback_rate <= 0.75 {
            AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_LOCAL.saturating_add(2)
        } else {
            AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_LOCAL
        };
    }
    if playback_rate >= 1.5 {
        3
    } else if playback_rate >= 1.25 {
        4
    } else if playback_rate <= 0.75 {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT.saturating_add(2)
    } else {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT
    }
}

pub fn audio_queue_prefill_target(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> usize {
    let base = if has_video_stream && is_realtime_source {
        AUDIO_QUEUE_PREFILL_REALTIME_VIDEO
    } else if has_video_stream {
        AUDIO_QUEUE_PREFILL_VIDEO
    } else if is_network_source {
        AUDIO_QUEUE_PREFILL_AUDIO_ONLY_NETWORK
    } else {
        AUDIO_QUEUE_PREFILL_AUDIO_ONLY_LOCAL
    };
    let rate = playback_rate.as_f32();
    if has_video_stream {
        if rate >= 1.5 {
            // Fast video playback is sensitive to short demux stalls; keep a healthier PCM runway.
            base.saturating_add(3)
        } else if rate >= 1.25 {
            base.saturating_add(2)
        } else if rate <= 0.75 {
            base.saturating_add(2)
        } else {
            base
        }
    } else if rate >= 1.5 {
        base.saturating_sub(1).max(2)
    } else if rate <= 0.75 {
        base.saturating_add(2)
    } else {
        base
    }
}

pub fn audio_queue_seconds_limit(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> Option<f64> {
    if is_realtime_source {
        return None;
    }
    let playback_rate = playback_rate.as_f32();
    Some(if has_video_stream {
        if playback_rate >= 1.25 {
            AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_FAST
        } else if playback_rate <= 0.75 {
            AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_SLOW
        } else {
            AUDIO_QUEUE_SECONDS_LIMIT_VIDEO_DEFAULT
        }
    } else if is_network_source {
        return None;
    } else if playback_rate >= 1.25 {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_FAST
    } else if playback_rate <= 0.75 {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_SLOW
    } else {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_DEFAULT
    })
}

pub fn audio_queue_refill_floor_seconds(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> Option<f64> {
    let queue_seconds_limit = audio_queue_seconds_limit(
        playback_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    )?;
    Some(if has_video_stream {
        (queue_seconds_limit * 0.9).max(0.09)
    } else if is_network_source {
        (queue_seconds_limit * 0.8).max(0.12)
    } else {
        (queue_seconds_limit * 0.75).max(0.06)
    })
}

pub fn seek_settle_queue_depth_limit(
    default_limit: usize,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> usize {
    if is_realtime_source {
        return default_limit;
    }
    let settle_limit = if has_video_stream {
        AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_VIDEO
    } else if is_network_source {
        AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_NETWORK
    } else {
        AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_LOCAL
    };
    default_limit.min(settle_limit.max(1))
}

fn realtime_audio_queue_depth_limit(playback_rate: PlaybackRate) -> usize {
    let playback_rate = playback_rate.as_f32();
    if playback_rate >= 1.5 {
        8
    } else if playback_rate >= 1.25 {
        10
    } else if playback_rate <= 0.75 {
        14
    } else {
        12
    }
}

