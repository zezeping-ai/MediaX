use crate::app::media::playback::runtime::audio_pipeline::{
    PlaybackHeadPosition, PlaybackHeadPrecision,
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncClockSource {
    AudioEstimated,
    AudioMeasured,
    VideoEstimated,
    VideoMeasured,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SyncClockSample {
    pub seconds: f64,
    pub source: SyncClockSource,
}

impl SyncClockSample {
    pub(crate) fn new(seconds: f64, source: SyncClockSource) -> Option<Self> {
        seconds
            .is_finite()
            .then_some(seconds.max(0.0))
            .map(|seconds| Self { seconds, source })
    }

    pub(crate) fn from_audio_output_position(position: PlaybackHeadPosition) -> Option<Self> {
        let source = match position.precision {
            PlaybackHeadPrecision::Measured => SyncClockSource::AudioMeasured,
            PlaybackHeadPrecision::Estimated => SyncClockSource::AudioEstimated,
        };
        Self::new(position.seconds, source)
    }
}

/// If device-measured head disagrees with queue-derived estimate beyond this, treat measurement as bad.
const MEASURED_VS_ESTIMATED_MAX_DRIFT_SECONDS: f64 = 0.35;

pub(crate) fn build_audio_sync_faces(
    audio_output_paused: bool,
    audio_clock_now_seconds: Option<f64>,
    observed_audio_clock: Option<SyncClockSample>,
) -> (Option<SyncClockSample>, Option<SyncClockSample>) {
    if audio_output_paused {
        return (None, None);
    }
    let estimated_audio_anchor = audio_clock_now_seconds
        .and_then(|seconds| SyncClockSample::new(seconds, SyncClockSource::AudioEstimated));
    let measured_audio_anchor = observed_audio_clock
        .filter(|sample| sample.source == SyncClockSource::AudioMeasured);
    (estimated_audio_anchor, measured_audio_anchor)
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PlaybackClockInput {
    pub hinted_seconds: Option<f64>,
    /// Queue / `AudioClock` anchor (always `AudioEstimated` semantics).
    pub estimated_audio_anchor: Option<SyncClockSample>,
    /// Device timeline when backend reports `AudioMeasured` (e.g. CPAL). Omit for estimate-only backends.
    pub measured_audio_anchor: Option<SyncClockSample>,
    pub critically_low_audio_buffer: bool,
    pub scheduling_lead_seconds: f64,
}

impl PlaybackClockInput {
    /// Scheduling and starved-hold both use: measured when trustworthy, else estimated.
    pub(crate) fn primary_audio_anchor(&self) -> Option<SyncClockSample> {
        let estimated = self
            .estimated_audio_anchor
            .filter(|s| s.seconds.is_finite() && s.seconds >= 0.0);
        let measured = self
            .measured_audio_anchor
            .filter(|s| s.source == SyncClockSource::AudioMeasured)
            .filter(|s| s.seconds.is_finite() && s.seconds >= 0.0);

        let Some(m) = measured else {
            return estimated;
        };
        let Some(e) = estimated else {
            return Some(m);
        };
        if (m.seconds - e.seconds).abs() <= MEASURED_VS_ESTIMATED_MAX_DRIFT_SECONDS {
            Some(m)
        } else {
            Some(e)
        }
    }

    fn anchor_plus_lead(sample: SyncClockSample, lead_seconds: f64) -> f64 {
        (sample.seconds + lead_seconds.max(0.0)).max(0.0)
    }

    pub(crate) fn scheduling_target_seconds(self) -> Option<f64> {
        if self.critically_low_audio_buffer {
            return None;
        }
        self.primary_audio_anchor()
            .map(|sample| Self::anchor_plus_lead(sample, self.scheduling_lead_seconds))
    }

    /// When the output queue is starved, hold the decode clock to the same primary anchor (+ lead) as steady state.
    pub(crate) fn starved_hold_target_seconds(self) -> Option<f64> {
        self.primary_audio_anchor()
            .map(|sample| Self::anchor_plus_lead(sample, self.scheduling_lead_seconds))
    }

    /// Best-effort audible head for logging/asserts: measured if used as primary, else estimated seconds.
    pub(crate) fn composite_head_seconds(self) -> Option<f64> {
        self.primary_audio_anchor().map(|s| s.seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackClockInput, SyncClockSample, SyncClockSource};

    #[test]
    fn scheduling_target_uses_estimated_anchor_and_lead() {
        let input = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(12.5, SyncClockSource::AudioEstimated),
            scheduling_lead_seconds: 0.008,
            ..PlaybackClockInput::default()
        };
        assert_eq!(input.scheduling_target_seconds(), Some(12.508));
    }

    #[test]
    fn scheduling_target_stops_when_audio_buffer_is_critically_low() {
        let input = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(12.5, SyncClockSource::AudioEstimated),
            critically_low_audio_buffer: true,
            scheduling_lead_seconds: 0.008,
            ..PlaybackClockInput::default()
        };
        assert_eq!(input.scheduling_target_seconds(), None);
    }

    #[test]
    fn primary_prefers_measured_when_close_to_estimated() {
        let input = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(10.0, SyncClockSource::AudioEstimated),
            measured_audio_anchor: SyncClockSample::new(10.02, SyncClockSource::AudioMeasured),
            scheduling_lead_seconds: 0.0,
            ..PlaybackClockInput::default()
        };
        assert_eq!(
            input.primary_audio_anchor(),
            SyncClockSample::new(10.02, SyncClockSource::AudioMeasured)
        );
    }

    #[test]
    fn primary_falls_back_to_estimated_when_measured_diverges() {
        let input = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(10.0, SyncClockSource::AudioEstimated),
            measured_audio_anchor: SyncClockSample::new(12.0, SyncClockSource::AudioMeasured),
            scheduling_lead_seconds: 0.0,
            ..PlaybackClockInput::default()
        };
        assert_eq!(
            input.primary_audio_anchor(),
            SyncClockSample::new(10.0, SyncClockSource::AudioEstimated)
        );
    }

    #[test]
    fn starved_hold_matches_primary_plus_lead() {
        let input = PlaybackClockInput {
            estimated_audio_anchor: SyncClockSample::new(5.0, SyncClockSource::AudioEstimated),
            measured_audio_anchor: SyncClockSample::new(5.01, SyncClockSource::AudioMeasured),
            critically_low_audio_buffer: true,
            scheduling_lead_seconds: 0.02,
            ..PlaybackClockInput::default()
        };
        let hold = input.starved_hold_target_seconds().expect("hold");
        assert!((hold - 5.03).abs() < 1e-6, "expected ~5.03 got {hold}");
    }

    #[test]
    fn measured_only_anchor_is_primary() {
        let input = PlaybackClockInput {
            measured_audio_anchor: SyncClockSample::new(3.0, SyncClockSource::AudioMeasured),
            ..PlaybackClockInput::default()
        };
        assert_eq!(
            input.primary_audio_anchor(),
            SyncClockSample::new(3.0, SyncClockSource::AudioMeasured)
        );
    }

    #[test]
    fn build_faces_treats_only_audio_measured_as_device_head() {
        let rodio_like = SyncClockSample::new(9.0, SyncClockSource::AudioEstimated).expect("sample");
        let (est, ms) = super::build_audio_sync_faces(false, Some(9.1), Some(rodio_like));
        assert!(ms.is_none());
        assert_eq!(
            est,
            SyncClockSample::new(9.1, SyncClockSource::AudioEstimated)
        );
    }
}
