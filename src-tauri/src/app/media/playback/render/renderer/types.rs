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
    pub y_plane: Vec<u8>,
    pub uv_plane: Vec<u8>,
    pub color_matrix: [[f32; 3]; 3],
    pub y_offset: f32,
    pub y_scale: f32,
    pub uv_offset: f32,
    pub uv_scale: f32,
}

#[derive(Clone, Copy, Default)]
pub struct RendererMetricsSnapshot {
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub last_render_cost_ms: f64,
    pub last_present_lag_ms: f64,
    pub last_presented_pts_seconds: Option<f64>,
    pub last_submitted_pts_seconds: Option<f64>,
    pub submit_lead_ms: f64,
}
