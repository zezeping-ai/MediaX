use crate::app::media::model::MediaLyricLine;

pub fn plain_text_to_timed_lines(text: &str, duration_seconds: f64) -> Vec<MediaLyricLine> {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let duration = duration_seconds.max(1.0);
    let step = duration / lines.len() as f64;
    lines
        .into_iter()
        .enumerate()
        .map(|(index, text)| MediaLyricLine {
            time_seconds: index as f64 * step,
            text: text.to_string(),
        })
        .collect()
}
