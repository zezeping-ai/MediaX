use crate::app::media::model::{MediaLibraryState, MediaSnapshot};
use crate::app::media::playback::dto::PlaybackState;

use super::model::PlaybackSessionModel;

pub(super) fn export_playback_state(model: &PlaybackSessionModel) -> PlaybackState {
    PlaybackState {
        engine: model.engine.clone(),
        status: model.transport.status.clone(),
        media_kind: model.source.media_kind,
        current_path: model.source.current_path.clone(),
        position_seconds: model.transport.position_seconds,
        duration_seconds: model.transport.duration_seconds,
        buffered_position_seconds: model.transport.buffered_position_seconds,
        playback_rate: model.transport.playback_rate.as_f64(),
        error: model.transport.error.clone(),
        hw_decode_mode: model.decode.hw_decode_mode,
        hw_decode_active: model.decode.hw_decode_active,
        hw_decode_backend: model.decode.hw_decode_backend.clone(),
        hw_decode_error: model.decode.hw_decode_error.clone(),
        quality_mode: model.source.quality_mode,
        adaptive_quality_supported: model.source.adaptive_quality_supported,
        volume: model.audio.volume,
        muted: model.audio.muted,
        left_channel_volume: model.audio.left_channel_volume,
        right_channel_volume: model.audio.right_channel_volume,
        left_channel_muted: model.audio.left_channel_muted,
        right_channel_muted: model.audio.right_channel_muted,
        channel_routing: model.audio.channel_routing,
    }
}

pub(super) fn export_media_snapshot(
    model: &PlaybackSessionModel,
    library: MediaLibraryState,
) -> MediaSnapshot {
    MediaSnapshot {
        playback: export_playback_state(model),
        library,
    }
}
