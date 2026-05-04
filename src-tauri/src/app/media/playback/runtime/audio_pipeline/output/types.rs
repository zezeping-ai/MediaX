#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaybackHeadPrecision {
    #[allow(dead_code)]
    Measured,
    Estimated,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaybackHeadPosition {
    pub seconds: f64,
    pub precision: PlaybackHeadPrecision,
}

impl PlaybackHeadPosition {
    pub(crate) fn estimated(seconds: f64) -> Self {
        Self {
            seconds: seconds.max(0.0),
            precision: PlaybackHeadPrecision::Estimated,
        }
    }
}
