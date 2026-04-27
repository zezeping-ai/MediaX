use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::playback::rate::{PlaybackRate, MAX_PLAYBACK_RATE, MIN_PLAYBACK_RATE};

pub const MIN_PREVIEW_EDGE: u32 = 32;
pub const MAX_PREVIEW_EDGE: u32 = 4096;

pub fn normalize_non_negative(value: f64, field: &str) -> MediaResult<f64> {
    if !value.is_finite() {
        return Err(MediaError::invalid_input(format!(
            "{field} must be a finite number"
        )));
    }
    Ok(value.max(0.0))
}

pub fn normalize_unit_interval(value: f64, field: &str) -> MediaResult<f64> {
    if !value.is_finite() {
        return Err(MediaError::invalid_input(format!(
            "{field} must be a finite number"
        )));
    }
    Ok(value.clamp(0.0, 1.0))
}

pub fn normalize_playback_rate(playback_rate: f64) -> MediaResult<PlaybackRate> {
    if !playback_rate.is_finite() {
        return Err(MediaError::invalid_input(
            "playback_rate must be a finite number",
        ));
    }
    Ok(PlaybackRate::from_f64(
        playback_rate.clamp(f64::from(MIN_PLAYBACK_RATE), f64::from(MAX_PLAYBACK_RATE)),
    ))
}

pub fn normalize_preview_edge(edge: u32) -> u32 {
    edge.clamp(MIN_PREVIEW_EDGE, MAX_PREVIEW_EDGE)
}
