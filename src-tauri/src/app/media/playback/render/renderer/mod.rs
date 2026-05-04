use std::sync::atomic::Ordering;

mod frame_queue;
mod helpers;
mod playback_head;
mod renderer_frame_ops;
mod renderer_init;
mod renderer_present;
mod renderer_state;
mod renderer_types;
mod types;

pub(crate) use playback_head::resolve_playback_references;
pub use renderer_state::RendererState;
pub(crate) use types::DecodedVideoFrame;
use types::QueuedFrame;
pub use types::{
    RendererMetricsSnapshot, VideoFrame, VideoFramePlanes, VideoPlaybackHeads, VideoScaleMode,
    VideoSyncReference,
};

use self::renderer_types::{ColorParams, Renderer};

impl RendererState {
    pub fn metrics_snapshot(&self) -> RendererMetricsSnapshot {
        let (queue_depth, queue_capacity, queued_head_pts_seconds, queued_tail_pts_seconds) =
            self.queue_metrics_snapshot();
        let last_presented_pts_seconds = self.last_presented_pts_seconds();
        let last_submitted_pts_seconds =
            queued_tail_pts_seconds.or_else(|| self.last_submitted_pts_seconds());
        let playback_heads = self.playback_heads();
        let submit_lead_ms = match (last_submitted_pts_seconds, last_presented_pts_seconds) {
            (Some(submitted), Some(presented))
                if submitted.is_finite() && presented.is_finite() && submitted >= presented =>
            {
                (submitted - presented) * 1000.0
            }
            _ => 0.0,
        };
        RendererMetricsSnapshot {
            queue_depth,
            queue_capacity,
            queued_head_pts_seconds,
            queued_tail_pts_seconds,
            last_render_cost_ms: (self.inner.last_render_cost_micros.load(Ordering::Relaxed)
                as f64)
                / 1000.0,
            last_present_lag_ms: f32::from_bits(
                self.inner.last_present_lag_ms_bits.load(Ordering::Relaxed),
            ) as f64,
            effective_display_pts_seconds: playback_heads
                .estimated
                .map(|position| position.seconds),
            last_presented_pts_seconds,
            last_submitted_pts_seconds,
            submit_lead_ms,
            render_loop_wakeups: self.inner.render_loop_wakeups.load(Ordering::Relaxed),
            render_attempts: self.inner.render_attempts.load(Ordering::Relaxed),
            render_presents: self.inner.render_presents.load(Ordering::Relaxed),
            render_uploads: self.inner.render_uploads.load(Ordering::Relaxed),
            playback_heads,
        }
    }
}
