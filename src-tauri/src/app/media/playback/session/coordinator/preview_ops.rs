use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::PreviewFrame;
use crate::app::media::playback::render::preview::generate_preview_frame;
use crate::app::media::playback::session::constraints;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use std::sync::atomic::Ordering;
use tauri::{async_runtime, AppHandle, Manager, State};

pub async fn preview_frame(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> MediaResult<Option<PreviewFrame>> {
    let target = constraints::normalize_non_negative(position_seconds, "position_seconds")?;
    let source = {
        let playback = state::playback(&state)?;
        playback.state().current_path
    };
    let Some(source) = source else {
        return Ok(None);
    };

    let epoch = state
        .runtime
        .preview_frame_epoch
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    let width = constraints::normalize_preview_edge(max_width.unwrap_or(160));
    let height = constraints::normalize_preview_edge(max_height.unwrap_or(90));
    let app_handle = app.clone();
    async_runtime::spawn_blocking(move || {
        generate_preview_frame(&source, target, width, height, || {
            app_handle
                .state::<MediaState>()
                .runtime
                .preview_frame_epoch
                .load(Ordering::Relaxed)
                != epoch
        })
    })
    .await
    .map_err(|err| MediaError::internal(format!("preview task join failed: {err}")))?
    .map_err(MediaError::from)
}
