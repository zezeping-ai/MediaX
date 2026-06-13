use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::AudioClock;

/// Audio-master presentation time used by the renderer. Falls back to seek target / zero when
/// audio is not yet available (video-only startup).
pub fn presentation_clock_seconds(
    audio_clock: Option<AudioClock>,
    active_seek_target_seconds: Option<f64>,
) -> f64 {
    if let Some(seconds) = audio_clock
        .map(|clock| clock.now_seconds())
        .filter(|value| value.is_finite() && *value >= 0.0)
    {
        return seconds;
    }
    active_seek_target_seconds.unwrap_or(0.0).max(0.0)
}

pub fn publish_presentation_clock(
    renderer: &RendererState,
    audio_clock: Option<AudioClock>,
    active_seek_target_seconds: Option<f64>,
    playback_rate: f64,
) {
    let media_seconds = presentation_clock_seconds(audio_clock, active_seek_target_seconds);
    renderer.update_clock(media_seconds, playback_rate);
}
