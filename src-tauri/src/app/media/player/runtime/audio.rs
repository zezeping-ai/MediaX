pub fn clamp_playback_rate(rate: f32) -> f32 {
    if rate.is_finite() {
        rate.max(0.25)
    } else {
        1.0
    }
}
