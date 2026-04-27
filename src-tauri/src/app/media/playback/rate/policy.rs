use super::value::PlaybackRate;

pub const RATE_SWITCH_SETTLE_WINDOW_MS: u64 = 320;

const BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT: usize = 24;
const RATE_SWITCH_DRAIN_THRESHOLD_SMALL_DELTA: usize = 2;
const RATE_SWITCH_DRAIN_THRESHOLD_LARGE_DELTA: usize = 1;

const DISCONTINUITY_FADE_IN_FRAMES_BASE: usize = 320;
const DISCONTINUITY_FADE_IN_FRAMES_MAX: usize = 640;
const DISCONTINUITY_CROSSFADE_FRAMES_BASE: usize = 256;
const DISCONTINUITY_CROSSFADE_FRAMES_MAX: usize = 512;

const OUTPUT_STAGING_FRAMES_FAST: usize = 768;
const OUTPUT_STAGING_FRAMES_DEFAULT: usize = 1024;
const OUTPUT_STAGING_FRAMES_SLOW: usize = 1280;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscontinuitySmoothingProfile {
    pub fade_in_frames: usize,
    pub crossfade_frames: usize,
}

pub fn rate_switch_queue_drain_threshold(
    current_rate: PlaybackRate,
    target_rate: PlaybackRate,
) -> usize {
    if current_rate.delta(target_rate) >= 0.5 {
        RATE_SWITCH_DRAIN_THRESHOLD_LARGE_DELTA
    } else {
        RATE_SWITCH_DRAIN_THRESHOLD_SMALL_DELTA
    }
}

pub fn audio_queue_depth_limit(playback_rate: PlaybackRate) -> usize {
    let playback_rate = playback_rate.as_f32();
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

pub fn output_staging_frames(playback_rate: PlaybackRate) -> usize {
    let playback_rate = playback_rate.as_f32();
    if playback_rate >= 1.25 {
        OUTPUT_STAGING_FRAMES_FAST
    } else if playback_rate <= 0.75 {
        OUTPUT_STAGING_FRAMES_SLOW
    } else {
        OUTPUT_STAGING_FRAMES_DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::{
        audio_queue_depth_limit, discontinuity_smoothing_profile, output_staging_frames,
        rate_switch_queue_drain_threshold, PlaybackRate, DISCONTINUITY_CROSSFADE_FRAMES_BASE,
        DISCONTINUITY_FADE_IN_FRAMES_BASE, OUTPUT_STAGING_FRAMES_DEFAULT,
        OUTPUT_STAGING_FRAMES_FAST, OUTPUT_STAGING_FRAMES_SLOW,
    };

    #[test]
    fn large_rate_switches_wait_for_tighter_queue_drain() {
        assert_eq!(
            rate_switch_queue_drain_threshold(PlaybackRate::new(1.0), PlaybackRate::new(1.6)),
            1
        );
        assert_eq!(
            rate_switch_queue_drain_threshold(PlaybackRate::new(1.0), PlaybackRate::new(1.2)),
            2
        );
    }

    #[test]
    fn queue_depth_policy_tracks_playback_rate() {
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.5)), 14);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(1.25)), 18);
        assert_eq!(audio_queue_depth_limit(PlaybackRate::new(0.75)), 30);
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
            output_staging_frames(PlaybackRate::new(1.0)),
            OUTPUT_STAGING_FRAMES_DEFAULT
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(1.5)),
            OUTPUT_STAGING_FRAMES_FAST
        );
        assert_eq!(
            output_staging_frames(PlaybackRate::new(0.5)),
            OUTPUT_STAGING_FRAMES_SLOW
        );
    }
}
