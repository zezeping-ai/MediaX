use super::super::helpers::set_pending_seek;
use crate::app::media::error::{MediaError, MediaResult};
use crate::app::media::model::MediaSnapshot;
use crate::app::media::playback::dto::{PlaybackMediaKind, PlaybackStatus};
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::render::viewport_sync;
use crate::app::media::playback::runtime::{emit_debug, write_latest_stream_position};
use crate::app::media::playback::session::constraints;
use crate::app::media::playback::session::service::supports_timeline_seek;
use crate::app::media::state;
use crate::app::media::state::MediaState;
use crate::app::media::state::emit_snapshot_with_request_id;
use tauri::{AppHandle, Manager, State};

pub fn seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    _force_render: Option<bool>,
    request_id: Option<String>,
) -> MediaResult<MediaSnapshot> {
    let position_seconds =
        constraints::normalize_non_negative(position_seconds, "position_seconds")?;
    let (media_path, status, media_kind, current_position_seconds) = {
        let mut playback = state::playback(&state)?;
        let playback_state = playback.state();
        if playback_state
            .current_path
            .as_deref()
            .is_some_and(|source| supports_timeline_seek(source))
        {
            playback.seek(position_seconds);
        }
        let next_state = playback.state();
        (
            next_state.current_path,
            next_state.status,
            next_state.media_kind,
            next_state.position_seconds,
        )
    };
    let supports_seek = media_path.as_deref().is_some_and(supports_timeline_seek);
    if !supports_seek {
        if let Some(source) = media_path.as_deref() {
            emit_debug(
                &app,
                "seek_ignored",
                format!(
                    "ignore seek for seek-limited source target={position_seconds:.3}s current={current_position_seconds:.3}s source={source}"
                ),
            );
        }
        return emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from);
    }
    if let Some(path_ref) = media_path.as_deref() {
        let mut library = state::library(&state)?;
        library.mark_playback_progress(path_ref, position_seconds);
    }
    set_pending_seek(&state, position_seconds)?;
    write_latest_stream_position(&state, position_seconds.max(0.0))?;
    apply_paused_seek_preview(
        &app,
        &state,
        media_path.as_deref(),
        &status,
        &media_kind,
        position_seconds,
    );
    emit_snapshot_with_request_id(&app, &state, request_id).map_err(MediaError::from)
}

fn apply_paused_seek_preview(
    app: &AppHandle,
    state: &State<'_, MediaState>,
    source: Option<&str>,
    status: &PlaybackStatus,
    media_kind: &PlaybackMediaKind,
    position_seconds: f64,
) {
    if *status != PlaybackStatus::Paused || *media_kind != PlaybackMediaKind::Video {
        return;
    }
    let Some(source) = source else {
        return;
    };
    let target = position_seconds.max(0.0);
    let renderer = (*app.state::<RendererState>()).clone();
    if let Err(err) = viewport_sync::sync_main_viewport_to(state, &renderer, source, target) {
        eprintln!("paused seek preview failed: {err}");
    }
}
