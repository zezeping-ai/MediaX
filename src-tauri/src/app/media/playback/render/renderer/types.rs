use ffmpeg_next::frame;

#[derive(Clone, Copy, Debug)]
pub enum VideoScaleMode {
    Contain,
    Cover,
}

impl VideoScaleMode {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Cover,
            _ => Self::Contain,
        }
    }

    pub fn as_u8(self) -> u8 {
        match self {
            Self::Contain => 0,
            Self::Cover => 1,
        }
    }
}

impl TryFrom<&str> for VideoScaleMode {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_ascii_lowercase().as_str() {
            "contain" => Ok(Self::Contain),
            "cover" => Ok(Self::Cover),
            other => Err(format!("unsupported video scale mode: {other}")),
        }
    }
}

#[derive(Clone)]
pub struct VideoFrame {
    pub pts_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub plane_strides: [u32; 2],
    pub planes: VideoFramePlanes,
    pub color_matrix: [[f32; 3]; 3],
    pub y_offset: f32,
    pub y_scale: f32,
    pub uv_offset: f32,
    pub uv_scale: f32,
}

pub(crate) struct DecodedVideoFrame {
    pub pts_seconds: f64,
    pub frame: frame::Video,
    pub color_matrix: [[f32; 3]; 3],
    pub y_offset: f32,
    pub y_scale: f32,
    pub uv_offset: f32,
    pub uv_scale: f32,
}

pub(super) enum QueuedFrame {
    Prepared(VideoFrame),
    Decoded(DecodedVideoFrame),
}

#[derive(Clone)]
pub enum VideoFramePlanes {
    Nv12 {
        y_plane: Vec<u8>,
        uv_plane: Vec<u8>,
    },
}

impl QueuedFrame {
    pub(super) fn pts_seconds(&self) -> f64 {
        match self {
            Self::Prepared(frame) => frame.pts_seconds,
            Self::Decoded(frame) => frame.pts_seconds,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct RendererMetricsSnapshot {
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub queued_head_pts_seconds: Option<f64>,
    pub queued_tail_pts_seconds: Option<f64>,
    pub last_render_cost_ms: f64,
    pub last_present_lag_ms: f64,
    pub effective_display_pts_seconds: Option<f64>,
    pub last_presented_pts_seconds: Option<f64>,
    pub last_submitted_pts_seconds: Option<f64>,
    pub submit_lead_ms: f64,
    pub render_loop_wakeups: u64,
    pub render_attempts: u64,
    pub render_presents: u64,
    pub render_uploads: u64,
    pub playback_heads: VideoPlaybackHeads,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoPlaybackHeadPrecision {
    Measured,
    Estimated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoPlaybackHeadSource {
    Presented,
    EffectiveDisplay,
}

#[derive(Clone, Copy, Debug)]
pub struct VideoPlaybackHeadPosition {
    pub seconds: f64,
    pub precision: VideoPlaybackHeadPrecision,
    pub source: VideoPlaybackHeadSource,
}

impl VideoPlaybackHeadPosition {
    pub fn precision_label(self) -> &'static str {
        match self.precision {
            VideoPlaybackHeadPrecision::Measured => "measured",
            VideoPlaybackHeadPrecision::Estimated => "estimated",
        }
    }

    pub fn source_label(self) -> &'static str {
        match self.source {
            VideoPlaybackHeadSource::Presented => "presented",
            VideoPlaybackHeadSource::EffectiveDisplay => "effective-display",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VideoPlaybackHeads {
    pub measured: Option<VideoPlaybackHeadPosition>,
    pub estimated: Option<VideoPlaybackHeadPosition>,
}

impl VideoPlaybackHeads {
    pub fn preferred(self) -> Option<VideoPlaybackHeadPosition> {
        self.measured.or(self.estimated)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoSyncFallbackKind {
    EstimatedPts,
}

#[derive(Clone, Copy, Debug)]
pub struct VideoSyncReference {
    pub position: VideoPlaybackHeadPosition,
    pub fallback: Option<VideoSyncFallbackKind>,
}

impl VideoSyncReference {
    pub fn precision_label(self) -> &'static str {
        self.position.precision_label()
    }

    pub fn source_label(self) -> &'static str {
        self.position.source_label()
    }

    pub fn fallback_label(self) -> &'static str {
        match self.fallback {
            Some(VideoSyncFallbackKind::EstimatedPts) => " fallback=estimated-pts",
            None => "",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VideoDisplayReference {
    pub position: Option<VideoPlaybackHeadPosition>,
}

impl VideoDisplayReference {
    pub fn seconds(self) -> Option<f64> {
        self.position.map(|position| position.seconds)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VideoPlaybackReferenceSet {
    pub sync: VideoSyncReference,
    pub display: VideoDisplayReference,
}

impl VideoPlaybackReferenceSet {
    pub fn display_video_pts_seconds(self) -> f64 {
        self.display
            .seconds()
            .unwrap_or(self.sync.position.seconds)
    }
}
