use crate::app::media::playback::dto::{PlaybackMediaKind, PlaybackQualityMode};

use super::model::PlaybackSessionModel;

pub(super) fn reset_playback_metrics(model: &mut PlaybackSessionModel) {
    model.transport.position_seconds = 0.0;
    model.transport.duration_seconds = 0.0;
    model.transport.buffered_position_seconds = 0.0;
}

pub(super) fn reset_runtime_decode_state(model: &mut PlaybackSessionModel) {
    model.decode.hw_decode_active = false;
    model.decode.hw_decode_backend = None;
    model.decode.hw_decode_error = None;
}

pub(super) fn reset_source_playback_state(model: &mut PlaybackSessionModel) {
    model.source.current_path = None;
    model.source.media_kind = PlaybackMediaKind::Video;
    reset_playback_metrics(model);
    model.transport.error = None;
    reset_runtime_decode_state(model);
    model.source.quality_mode = PlaybackQualityMode::Source;
    model.source.adaptive_quality_supported = false;
}
