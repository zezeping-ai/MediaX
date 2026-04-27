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
        RendererMetricsSnapshot {
            queue_depth: self.queue_depth(),
            queue_capacity: renderer_state::FRAME_QUEUE_CAPACITY,
            last_render_cost_ms: (self.inner.last_render_cost_micros.load(Ordering::Relaxed)
                as f64)
                / 1000.0,
            last_present_lag_ms: f32::from_bits(
                self.inner.last_present_lag_ms_bits.load(Ordering::Relaxed),
            ) as f64,
        }
    }
}
