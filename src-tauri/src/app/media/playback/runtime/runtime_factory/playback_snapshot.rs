use crate::app::media::error::MediaError;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::state::MediaState;
use tauri::{AppHandle, Manager};

pub(super) fn update_hw_decode_snapshot(
    app: &AppHandle,
    video_ctx: &VideoDecodeContext,
) -> Result<(), String> {
    let media_state = app.state::<MediaState>();
    let mut playback = media_state
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
