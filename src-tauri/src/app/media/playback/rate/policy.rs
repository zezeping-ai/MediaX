use super::value::PlaybackRate;

pub const RATE_SWITCH_SETTLE_WINDOW_MS: u64 = 320;

const BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT: usize = 24;
const AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_LOCAL: usize = 8;
const AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_AUDIO_ONLY_NETWORK: usize = 16;

const DISCONTINUITY_FADE_IN_FRAMES_BASE: usize = 320;
const DISCONTINUITY_FADE_IN_FRAMES_MAX: usize = 640;
const DISCONTINUITY_CROSSFADE_FRAMES_BASE: usize = 256;
const DISCONTINUITY_CROSSFADE_FRAMES_MAX: usize = 512;

const OUTPUT_STAGING_FRAMES_FAST: usize = 768;
const OUTPUT_STAGING_FRAMES_DEFAULT: usize = 1024;
const OUTPUT_STAGING_FRAMES_SLOW: usize = 1280;
const OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_REALTIME: usize = 1024;
const OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_VIDEO: usize = 2048;
const OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_AUDIO_ONLY: usize = 4096;
const OUTPUT_STAGING_FRAMES_SEEK_REFILL_VIDEO: usize = 768;
const OUTPUT_STAGING_FRAMES_SEEK_REFILL_AUDIO_ONLY: usize = 512;
const OUTPUT_STAGING_FRAMES_SEEK_SETTLE_VIDEO: usize = 1024;
const OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_LOCAL: usize = 2048;
const OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_NETWORK: usize = 1024;
const AUDIO_QUEUE_PREFILL_VIDEO: usize = 5;
const AUDIO_QUEUE_PREFILL_AUDIO_ONLY_LOCAL: usize = 3;
const AUDIO_QUEUE_PREFILL_AUDIO_ONLY_NETWORK: usize = 6;
const AUDIO_QUEUE_PREFILL_REALTIME_VIDEO: usize = 3;
const VIDEO_DRAIN_BATCH_LIMIT_CRITICAL: usize = 1;
const VIDEO_DRAIN_BATCH_LIMIT_LOW: usize = 2;
const VIDEO_DRAIN_BATCH_LIMIT_WARMUP: usize = 3;
const AUDIO_RATE_SWITCH_COVER_SECONDS_REALTIME: f64 = 0.12;
const AUDIO_RATE_SWITCH_COVER_SECONDS_VIDEO: f64 = 0.30;
const AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_LOCAL: f64 = 0.32;
const AUDIO_RATE_SWITCH_COVER_SECONDS_AUDIO_ONLY_NETWORK: f64 = 1.20;
const AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_REALTIME: f64 = 0.08;
const AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_VIDEO: f64 = 0.16;
const AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_LOCAL: f64 = 0.24;
const AUDIO_RATE_SWITCH_MIN_APPLY_SECONDS_AUDIO_ONLY_NETWORK: f64 = 1.20;
const AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_FAST: f64 = 0.16;
const AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_DEFAULT: f64 = 0.18;
const AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_SLOW: f64 = 0.24;
const AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_LOCAL: usize = 6;
const AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_AUDIO_ONLY_NETWORK: usize = 8;
const AUDIO_QUEUE_SOURCE_DEPTH_LIMIT_SEEK_SETTLE_VIDEO: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscontinuitySmoothingProfile {
    pub fade_in_frames: usize,
    pub crossfade_frames: usize,
}

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
        14
    } else if playback_rate >= 1.25 {
        18
    } else if playback_rate <= 0.75 {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT.saturating_add(6)
    } else {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT
    }
}

pub fn discontinuity_smoothing_profile(
    previous_rate: PlaybackRate,
    next_rate: PlaybackRate,
) -> DiscontinuitySmoothingProfile {
    let delta = previous_rate.delta(next_rate).clamp(0.0, 2.0);
    let fade_in_scaled = (delta / 2.0)
        * (DISCONTINUITY_FADE_IN_FRAMES_MAX - DISCONTINUITY_FADE_IN_FRAMES_BASE) as f32;
    let crossfade_scaled = (delta / 2.0)
        * (DISCONTINUITY_CROSSFADE_FRAMES_MAX - DISCONTINUITY_CROSSFADE_FRAMES_BASE) as f32;
    DiscontinuitySmoothingProfile {
        fade_in_frames: DISCONTINUITY_FADE_IN_FRAMES_BASE + fade_in_scaled.round() as usize,
        crossfade_frames: DISCONTINUITY_CROSSFADE_FRAMES_BASE + crossfade_scaled.round() as usize,
    }
}

pub fn output_staging_frames(
    playback_rate: PlaybackRate,
    _has_video_stream: bool,
    _is_network_source: bool,
) -> usize {
    let playback_rate = playback_rate.as_f32();
    if playback_rate >= 1.25 {
        OUTPUT_STAGING_FRAMES_FAST
    } else if playback_rate <= 0.75 {
        OUTPUT_STAGING_FRAMES_SLOW
    } else {
        OUTPUT_STAGING_FRAMES_DEFAULT
    }
}

pub fn rate_switch_cover_output_staging_frames(
    has_video_stream: bool,
    is_realtime_source: bool,
) -> usize {
    if is_realtime_source {
        return OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_REALTIME;
    }
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_VIDEO
    } else {
        OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_AUDIO_ONLY
    }
}

pub fn seek_refill_output_staging_frames(has_video_stream: bool) -> usize {
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_SEEK_REFILL_VIDEO
    } else {
        OUTPUT_STAGING_FRAMES_SEEK_REFILL_AUDIO_ONLY
    }
}

pub fn seek_settle_output_staging_frames(
    has_video_stream: bool,
    is_network_source: bool,
) -> usize {
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_VIDEO
    } else if is_network_source {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_NETWORK
    } else {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_LOCAL
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
    if playback_rate.as_f32() >= 1.5 {
        base.saturating_sub(1).max(2)
    } else if playback_rate.as_f32() <= 0.75 {
        base.saturating_add(2)
    } else {
        base
    }
}

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

pub fn audio_queue_seconds_limit(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
) -> Option<f64> {
    if has_video_stream || is_realtime_source || is_network_source {
        return None;
    }
    let playback_rate = playback_rate.as_f32();
    Some(if playback_rate >= 1.25 {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_FAST
    } else if playback_rate <= 0.75 {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_SLOW
    } else {
        AUDIO_QUEUE_SECONDS_LIMIT_AUDIO_ONLY_LOCAL_DEFAULT
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

pub fn video_drain_batch_limit(
    playback_rate: PlaybackRate,
    has_audio_stream: bool,
    audio_queue_depth: Option<usize>,
    is_realtime_source: bool,
) -> Option<usize> {
    if !has_audio_stream {
        return None;
    }
    let prefill_target =
        audio_queue_prefill_target(playback_rate, true, is_realtime_source, false);
    let critical_threshold = (prefill_target / 2).max(1);
    match audio_queue_depth {
        None if is_realtime_source => Some(VIDEO_DRAIN_BATCH_LIMIT_CRITICAL),
        None => Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP),
        Some(depth) if depth <= critical_threshold => Some(VIDEO_DRAIN_BATCH_LIMIT_CRITICAL),
        Some(depth) if depth < prefill_target => Some(VIDEO_DRAIN_BATCH_LIMIT_LOW),
        Some(depth) if is_realtime_source && depth <= prefill_target.saturating_add(1) => {
            Some(VIDEO_DRAIN_BATCH_LIMIT_LOW)
        }
        Some(_) if is_realtime_source && playback_rate.as_f32() > 1.0 => {
            Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP)
        }
        Some(depth) if playback_rate.as_f32() >= 1.25 && depth <= prefill_target.saturating_add(1) => {
            Some(VIDEO_DRAIN_BATCH_LIMIT_WARMUP)
        }
        Some(_) => None,
    }
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

#[cfg(test)]
mod tests {
    use super::{
        audio_queue_depth_limit, audio_queue_prefill_target, audio_queue_seconds_limit,
        audio_rate_switch_cover_seconds, audio_rate_switch_min_apply_seconds,
        discontinuity_smoothing_profile, output_staging_frames,
        seek_refill_output_staging_frames, seek_settle_output_staging_frames,
        seek_settle_queue_depth_limit, rate_switch_cover_output_staging_frames,
        video_drain_batch_limit, PlaybackRate,
        DISCONTINUITY_CROSSFADE_FRAMES_BASE, DISCONTINUITY_FADE_IN_FRAMES_BASE,
        OUTPUT_STAGING_FRAMES_DEFAULT, OUTPUT_STAGING_FRAMES_FAST, OUTPUT_STAGING_FRAMES_SLOW,
    };

    #[test]
    fn queue_depth_policy_tracks_playback_rate() {
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.5), true, false, false), 14);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.25), true, false, false), 18);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(0.75), true, false, false), 30);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.25), true, true, true), 10);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.0), false, false, false), 8);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.0), false, false, true), 16);
    }

    #[test]
    fn discontinuity_smoothing_scales_with_rate_delta() {
        let neutral =
            discontinuity_smoothing_profile(PlaybackRate::new(1.0), PlaybackRate::new(1.0));
        let aggressive =
            discontinuity_smoothing_profile(PlaybackRate::new(1.0), PlaybackRate::new(2.0));
        assert_eq!(neutral.fade_in_frames, DISCONTINUITY_FADE_IN_FRAMES_BASE);
        assert!(aggressive.crossfade_frames > DISCONTINUITY_CROSSFADE_FRAMES_BASE);
    }

    #[test]
    fn output_staging_tracks_playback_rate() {
        assert_eq!(
            output_staging_frames(PlaybackRate::new(1.0), true, false),
            OUTPUT_STAGING_FRAMES_DEFAULT
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(1.5), true, false),
            OUTPUT_STAGING_FRAMES_FAST
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(0.5), true, false),
            OUTPUT_STAGING_FRAMES_SLOW
        );
    }

    #[test]
    fn audio_only_local_output_staging_stays_on_shared_defaults() {
        assert_eq!(
            output_staging_frames(PlaybackRate::new(1.0), false, false),
            OUTPUT_STAGING_FRAMES_DEFAULT
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(1.5), false, false),
            OUTPUT_STAGING_FRAMES_FAST
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(0.5), false, false),
            OUTPUT_STAGING_FRAMES_SLOW
        );
    }

    #[test]
    fn rate_switch_cover_output_uses_coarser_blocks() {
        assert_eq!(rate_switch_cover_output_staging_frames(true, true), 1024);
        assert_eq!(rate_switch_cover_output_staging_frames(true, false), 2048);
        assert_eq!(rate_switch_cover_output_staging_frames(false, false), 4096);
    }

    #[test]
    fn seek_refill_output_prefers_low_latency_blocks() {
        assert_eq!(seek_refill_output_staging_frames(true), 768);
        assert_eq!(seek_refill_output_staging_frames(false), 512);
    }

    #[test]
    fn seek_settle_output_uses_coarser_local_audio_blocks() {
        assert_eq!(seek_settle_output_staging_frames(true, false), 1024);
        assert_eq!(seek_settle_output_staging_frames(false, false), 2048);
        assert_eq!(seek_settle_output_staging_frames(false, true), 1024);
    }

    #[test]
    fn seek_settle_queue_limit_stays_tighter_than_default() {
        assert_eq!(seek_settle_queue_depth_limit(8, false, false, false), 6);
        assert_eq!(seek_settle_queue_depth_limit(16, false, false, true), 8);
        assert_eq!(seek_settle_queue_depth_limit(24, true, false, false), 8);
        assert_eq!(seek_settle_queue_depth_limit(10, true, true, true), 10);
    }

    #[test]
    fn audio_prefill_tracks_media_type_and_rate() {
        assert_eq!(audio_queue_prefill_target(PlaybackRate::new(1.0), true, false, false), 5);
        assert_eq!(audio_queue_prefill_target(PlaybackRate::new(1.0), false, false, false), 3);
        assert_eq!(audio_queue_prefill_target(PlaybackRate::new(1.0), false, false, true), 6);
        assert_eq!(audio_queue_prefill_target(PlaybackRate::new(0.5), false, false, false), 5);
        assert_eq!(audio_queue_prefill_target(PlaybackRate::new(1.0), true, true, true), 3);
    }

    #[test]
    fn rate_switch_cover_seconds_track_source_type() {
        assert!((audio_rate_switch_cover_seconds(false, false, false) - 0.32).abs() < 1e-6);
        assert!((audio_rate_switch_cover_seconds(false, false, true) - 1.20).abs() < 1e-6);
        assert!((audio_rate_switch_cover_seconds(true, false, false) - 0.30).abs() < 1e-6);
        assert!((audio_rate_switch_cover_seconds(true, true, true) - 0.12).abs() < 1e-6);
    }

    #[test]
    fn rate_switch_min_apply_seconds_track_source_type() {
        assert!((audio_rate_switch_min_apply_seconds(false, false, false) - 0.24).abs() < 1e-6);
        assert!((audio_rate_switch_min_apply_seconds(false, false, true) - 1.20).abs() < 1e-6);
        assert!((audio_rate_switch_min_apply_seconds(true, false, false) - 0.16).abs() < 1e-6);
        assert!((audio_rate_switch_min_apply_seconds(true, true, true) - 0.08).abs() < 1e-6);
    }

    #[test]
    fn local_audio_queue_seconds_limit_tracks_rate() {
        assert_eq!(
            audio_queue_seconds_limit(PlaybackRate::new(1.0), false, false, false),
            Some(0.18)
        );
        assert_eq!(
            audio_queue_seconds_limit(PlaybackRate::new(1.5), false, false, false),
            Some(0.16)
        );
        assert_eq!(
            audio_queue_seconds_limit(PlaybackRate::new(0.5), false, false, false),
            Some(0.24)
        );
        assert_eq!(
            audio_queue_seconds_limit(PlaybackRate::new(1.0), false, false, true),
            None
        );
    }

    #[test]
    fn video_drain_batch_limit_protects_audio_queue() {
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.0), false, None, false), None);
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.0), true, None, false), Some(3));
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(2), false), Some(1));
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(4), false), Some(2));
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.5), true, Some(5), false), Some(3));
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(8), false), None);
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.25), true, None, true), Some(1));
        assert_eq!(video_drain_batch_limit(PlaybackRate::new(1.25), true, Some(4), true), Some(2));
    }
}
