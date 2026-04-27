use crate::app::media::playback::dto::PlaybackQualityMode;

pub(super) fn compute_output_size(
    width: u32,
    height: u32,
    quality_mode: PlaybackQualityMode,
) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }
    let Some(max_height) = quality_mode_max_height(quality_mode) else {
        let mut out_width = width.max(2);
        let mut out_height = height.max(2);
        out_width &= !1;
        out_height &= !1;
        return (out_width.max(2), out_height.max(2));
    };
    let height_scale = (max_height as f64) / (height as f64);
    let scale = height_scale.min(1.0);
    let mut out_width = ((width as f64) * scale).round().max(2.0) as u32;
    let mut out_height = ((height as f64) * scale).round().max(2.0) as u32;
    out_width &= !1;
    out_height &= !1;
    (out_width.max(2), out_height.max(2))
}

fn quality_mode_max_height(mode: PlaybackQualityMode) -> Option<u32> {
    match mode {
        PlaybackQualityMode::Source | PlaybackQualityMode::Auto => None,
        PlaybackQualityMode::P1080 => Some(1080),
        PlaybackQualityMode::P720 => Some(720),
        PlaybackQualityMode::P480 => Some(480),
        PlaybackQualityMode::P320 => Some(320),
    }
}
