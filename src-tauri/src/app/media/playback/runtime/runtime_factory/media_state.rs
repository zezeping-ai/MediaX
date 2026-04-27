use crate::app::media::error::MediaError;
use crate::app::media::model::{PlaybackMediaKind, PlaybackQualityMode};
use crate::app::media::state::MediaState;

pub(super) fn resolve_quality_mode(media_state: &MediaState) -> Result<PlaybackQualityMode, String> {
    let playback = media_state
        .playback
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
    Ok(playback.quality_mode())
}

pub(super) fn sync_media_kind(
    media_state: &MediaState,
    media_kind: PlaybackMediaKind,
) -> Result<(), String> {
    let mut playback = media_state
        .playback
        .lock()
        .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
    playback.set_media_kind(media_kind);
    Ok(())
}
