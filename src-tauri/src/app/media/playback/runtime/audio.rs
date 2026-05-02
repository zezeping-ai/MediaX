use crate::app::media::playback::rate::PlaybackRate;

pub fn clamp_playback_rate(rate: f32) -> f32 {
    PlaybackRate::new(rate).as_f32()
}

pub fn playback_rate_limited_reason(
    rate: PlaybackRate,
    is_realtime_source: bool,
) -> Option<&'static str> {
    if is_realtime_source && rate.as_f32() > 1.0 {
        Some("realtime_source")
    } else {
        None
    }
}

pub fn effective_playback_rate(rate: PlaybackRate, is_realtime_source: bool) -> PlaybackRate {
    if playback_rate_limited_reason(rate, is_realtime_source).is_some() {
        PlaybackRate::new(1.0)
    } else {
        rate
    }
}
