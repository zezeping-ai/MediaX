use crate::app::media::playback::rate::{audio_queue_prefill_target, audio_queue_refill_floor_seconds};
use crate::app::media::playback::runtime::audio_pipeline::AudioDrainParams;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BackpressureClass {
    HighWater,
    NormalWater,
    LowWater,
}

impl BackpressureClass {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::HighWater => "high_water",
            Self::NormalWater => "normal_water",
            Self::LowWater => "low_water",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum SourceKind {
    AudioOnly,
    AvFile,
    AvRealtime,
    AvNetwork,
}

impl SourceKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::AudioOnly => "audio_only",
            Self::AvFile => "av_file",
            Self::AvRealtime => "av_realtime",
            Self::AvNetwork => "av_network",
        }
    }
}

pub(super) struct AudioBackpressureProfile {
    pub(super) class: BackpressureClass,
    pub(super) source_kind: SourceKind,
    pub(super) high_water: bool,
    pub(super) underrun_count: u64,
    pub(super) spin_limit: usize,
    pub(super) queue_depth: usize,
    pub(super) prefill_target: usize,
    pub(super) queued_seconds: f64,
    pub(super) refill_floor_seconds: f64,
}

pub(super) fn audio_backpressure_profile(
    audio_state: &crate::app::media::playback::runtime::audio_pipeline::AudioPipeline,
    params: AudioDrainParams,
) -> AudioBackpressureProfile {
    let prefill_target = audio_queue_prefill_target(
        params.applied_playback_rate,
        params.has_video_stream,
        params.is_realtime_source,
        params.is_network_source,
    );
    let refill_floor_seconds = audio_queue_refill_floor_seconds(
        params.applied_playback_rate,
        params.has_video_stream,
        params.is_realtime_source,
        params.is_network_source,
    )
    .unwrap_or(0.09);
    let queue_depth = audio_state.output.queue_depth();
    let queued_seconds = audio_state.output.queued_duration_seconds();
    let low_watermark = queue_depth < prefill_target && queued_seconds + 1e-3 < refill_floor_seconds;
    let high_watermark = queue_depth >= prefill_target.saturating_add(2)
        || queued_seconds > (refill_floor_seconds + 0.22);
    let mut limit = if low_watermark {
        super::super::AUDIO_SEND_PACKET_SPIN_LIMIT.saturating_add(1)
    } else if high_watermark {
        super::super::AUDIO_SEND_PACKET_SPIN_LIMIT
            .saturating_sub(1)
            .max(1)
    } else {
        super::super::AUDIO_SEND_PACKET_SPIN_LIMIT
    };
    if params.applied_playback_rate.as_f32() >= 1.5 {
        limit = limit.saturating_add(1);
    }
    let class = if low_watermark {
        BackpressureClass::LowWater
    } else if high_watermark {
        BackpressureClass::HighWater
    } else {
        BackpressureClass::NormalWater
    };
    let source_kind = if !params.has_video_stream {
        SourceKind::AudioOnly
    } else if params.is_realtime_source {
        SourceKind::AvRealtime
    } else if params.is_network_source {
        SourceKind::AvNetwork
    } else {
        SourceKind::AvFile
    };
    AudioBackpressureProfile {
        class,
        source_kind,
        high_water: high_watermark,
        underrun_count: audio_state.stats.underrun_count,
        spin_limit: limit.clamp(1, 4),
        queue_depth,
        prefill_target,
        queued_seconds,
        refill_floor_seconds,
    }
}
