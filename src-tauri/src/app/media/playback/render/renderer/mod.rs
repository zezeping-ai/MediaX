use std::sync::atomic::Ordering;

mod frame_queue;
mod helpers;
mod renderer_frame_ops;
mod renderer_init;
mod renderer_present;
mod renderer_state;
mod renderer_types;
mod types;

pub use types::{RendererMetricsSnapshot, VideoFrame, VideoScaleMode};
pub use renderer_state::RendererState;

use self::renderer_types::{ColorParams, Renderer};

impl RendererState {
    pub fn metrics_snapshot(&self) -> RendererMetricsSnapshot {
        let last_presented_pts_seconds = self.last_presented_pts_seconds();
        let last_submitted_pts_seconds = self.last_submitted_pts_seconds();
        let submit_lead_ms = match (last_submitted_pts_seconds, last_presented_pts_seconds) {
            (Some(submitted), Some(presented))
                if submitted.is_finite() && presented.is_finite() && submitted >= presented =>
            {
                (submitted - presented) * 1000.0
            }
            _ => 0.0,
        };
        RendererMetricsSnapshot {
            queue_depth: self.queue_depth(),
            queue_capacity: self.queue_capacity(),
            last_render_cost_ms: (self.inner.last_render_cost_micros.load(Ordering::Relaxed)
                as f64)
                / 1000.0,
            last_present_lag_ms: f32::from_bits(
                self.inner.last_present_lag_ms_bits.load(Ordering::Relaxed),
            ) as f64,
            last_presented_pts_seconds,
            last_submitted_pts_seconds,
            submit_lead_ms,
        }
    }
}
