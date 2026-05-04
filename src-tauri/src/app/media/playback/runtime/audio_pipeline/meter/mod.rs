mod accumulator;
mod shared;
mod source;
mod spectrum;

pub(crate) use accumulator::AudioMeterAccumulator;
pub(crate) use shared::{
    create_shared_audio_meter, notify_meter_shutdown, publish_snapshot, spawn_audio_meter_emitter,
    SharedAudioMeter,
};
pub(crate) use source::{MeteredSource, QueuedDurationTracker};
