//! Main-window viewport updates when the decode thread is not driving presentation.
//!
//! While **paused** or **stopped**, the FFmpeg decode worker is not running (or not
//! submitting frames). Any timeline change must decode a single preview frame and push
//! it to [`RendererState`]. This module owns that policy:
//!
//! - bumps [`MediaState::paused_seek_epoch`] so overlapping work can abort cleanly
//! - calls [`RendererState::reset_timeline`] before submit so backward seeks are not
//!   dropped by the renderer’s monotonic PTS guard
//! - decodes via [`crate::app::media::player::preview::render_preview_frame_at`]
//!
//! Future features (e.g. “frame step”, A/B stills) can reuse [`sync_main_viewport_to`]
//! instead of duplicating epoch / reset / preview wiring.

use std::sync::atomic::Ordering;

use crate::app::media::player::preview::render_preview_frame_at;
use crate::app::media::player::renderer::RendererState;
use crate::app::media::player::state::MediaState;

/// Decode `source` at `position_seconds` and present it on the main viewport.
///
/// Synchronous and intended for **paused / stopped** UI paths only.
pub fn sync_main_viewport_to(
    media: &MediaState,
    renderer: &RendererState,
    source: &str,
    position_seconds: f64,
) -> Result<(), String> {
    let epoch = media
        .paused_seek_epoch
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    let clamped = position_seconds.max(0.0);
    let rate = media.timing_controls.playback_rate() as f64;
    renderer.reset_timeline(clamped, rate);
    render_preview_frame_at(renderer, source, position_seconds, || {
        media.paused_seek_epoch.load(Ordering::Relaxed) != epoch
    })
}
