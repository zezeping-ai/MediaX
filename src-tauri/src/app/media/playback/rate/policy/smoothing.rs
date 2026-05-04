use super::constants::{
    DISCONTINUITY_CROSSFADE_FRAMES_BASE, DISCONTINUITY_CROSSFADE_FRAMES_MAX,
    DISCONTINUITY_FADE_IN_FRAMES_BASE, DISCONTINUITY_FADE_IN_FRAMES_MAX,
};
use super::super::value::PlaybackRate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscontinuitySmoothingProfile {
    pub fade_in_frames: usize,
    pub crossfade_frames: usize,
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

