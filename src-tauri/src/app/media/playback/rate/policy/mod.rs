mod audio_queue;
mod constants;
mod smoothing;
mod staging;
mod video;

pub use audio_queue::{
    audio_queue_depth_limit, audio_queue_prefill_target, audio_queue_refill_floor_seconds,
    audio_queue_seconds_limit, seek_settle_queue_depth_limit,
};
pub use constants::RATE_SWITCH_SETTLE_WINDOW_MS;
pub use smoothing::discontinuity_smoothing_profile;
pub use staging::{
    output_staging_frames, rate_switch_cover_output_staging_frames, seek_refill_output_staging_frames,
    seek_settle_output_staging_frames,
};
pub use video::{audio_rate_switch_cover_seconds, audio_rate_switch_min_apply_seconds, video_drain_batch_limit};

#[cfg(test)]
mod tests;

