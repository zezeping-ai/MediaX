use crate::app::media::playback::rate::PlaybackRate;

pub fn clamp_playback_rate(rate: f32) -> f32 {
    PlaybackRate::new(rate).as_f32()
}
