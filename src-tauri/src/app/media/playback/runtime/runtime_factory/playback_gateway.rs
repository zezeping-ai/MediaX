use crate::app::media::error::MediaError;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::playback::dto::{PlaybackMediaKind, PlaybackQualityMode};
use crate::app::media::state::MediaState;

pub(super) struct PlaybackRuntimeGateway<'a> {
    media_state: &'a MediaState,
}

impl<'a> PlaybackRuntimeGateway<'a> {
    pub(super) fn new(media_state: &'a MediaState) -> Self {
        Self { media_state }
    }

    pub(super) fn quality_mode(&self) -> Result<PlaybackQualityMode, String> {
        let playback = self
            .media_state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        Ok(playback.quality_mode())
    }

    pub(super) fn sync_media_kind(&self, media_kind: PlaybackMediaKind) -> Result<(), String> {
        let mut playback = self
            .media_state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.set_media_kind(media_kind);
        Ok(())
    }

    pub(super) fn sync_hw_decode_snapshot(
        &self,
        video_ctx: &VideoDecodeContext,
    ) -> Result<(), String> {
        let mut playback = self
            .media_state
            .session
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.update_hw_decode_status(
            video_ctx.hw_decode_active,
            video_ctx.hw_decode_backend.clone(),
            video_ctx.hw_decode_error.clone(),
        );
        Ok(())
    }
}
