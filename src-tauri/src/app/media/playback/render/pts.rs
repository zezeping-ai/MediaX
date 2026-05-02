use ffmpeg_next as ffmpeg;

pub fn to_seconds(ts: i64, time_base: ffmpeg::Rational) -> f64 {
    ts as f64 * f64::from(time_base)
}

pub fn timestamp_to_seconds(
    timestamp: Option<i64>,
    pts: Option<i64>,
    time_base: ffmpeg::Rational,
) -> Option<f64> {
    pts.or(timestamp).map(|ts| to_seconds(ts, time_base))
}
