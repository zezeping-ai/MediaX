use crate::app::media::model::{PlaybackQualityMode, PlaybackState};

pub(super) fn reset_playback_metrics(state: &mut PlaybackState) {
    state.position_seconds = 0.0;
    state.duration_seconds = 0.0;
}

pub(super) fn reset_runtime_decode_state(state: &mut PlaybackState) {
    state.hw_decode_active = false;
    state.hw_decode_backend = None;
    state.hw_decode_error = None;
}

pub(super) fn reset_source_playback_state(state: &mut PlaybackState) {
    state.current_path = None;
    reset_playback_metrics(state);
    state.error = None;
    reset_runtime_decode_state(state);
    state.quality_mode = PlaybackQualityMode::Source;
    state.adaptive_quality_supported = false;
}
