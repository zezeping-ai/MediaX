use crate::app::media::model::MediaLyricLine;

pub fn parse_lrc_contents(contents: &str) -> Vec<MediaLyricLine> {
    let mut lines = Vec::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let mut rest = line;
        let mut timestamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some((ts, tail)) = stripped.split_once(']') else {
                break;
            };
            let Some(seconds) = parse_lrc_timestamp(ts) else {
                break;
            };
            timestamps.push(seconds);
            rest = tail.trim_start();
        }
        if timestamps.is_empty() || rest.is_empty() {
            continue;
        }
        for timestamp in timestamps {
            lines.push(MediaLyricLine {
                time_seconds: timestamp,
                text: rest.to_string(),
            });
        }
    }
    lines.sort_by(|a, b| {
        a.time_seconds
            .partial_cmp(&b.time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    lines
}

fn parse_lrc_timestamp(value: &str) -> Option<f64> {
    let parts: Vec<&str> = value.split(':').collect();
    match parts.as_slice() {
        [minutes, seconds] => {
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(minutes * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours = hours.parse::<f64>().ok()?;
            let minutes = minutes.parse::<f64>().ok()?;
            let seconds = seconds.parse::<f64>().ok()?;
            Some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_lrc_contents;

    #[test]
    fn parses_centisecond_timestamps() {
        let lines = parse_lrc_contents("[00:17.12] Hello world\n[01:02.50] Next line");
        assert_eq!(lines.len(), 2);
        assert!((lines[0].time_seconds - 17.12).abs() < 0.001);
        assert_eq!(lines[0].text, "Hello world");
    }
}
