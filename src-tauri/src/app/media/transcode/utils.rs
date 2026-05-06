use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn source_basename(source: &str, fallback: &str) -> String {
    let no_query = source
        .split(['?', '#'])
        .next()
        .unwrap_or(source)
        .trim_end_matches('/');
    let raw_name = no_query
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    let stem = Path::new(raw_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(fallback);
    let sanitized = stem
        .chars()
        .map(|ch| if r#"\/:*?"<>|"#.contains(ch) { '-' } else { ch })
        .collect::<String>()
        .trim()
        .to_string();
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

pub fn next_available_output_path(
    output_dir: &str,
    source: &str,
    suffix: &str,
    extension: &str,
    fallback: &str,
) -> PathBuf {
    let base = source_basename(source, fallback);
    let mut candidate = Path::new(output_dir).join(format!("{base}{suffix}.{extension}"));
    if !candidate.exists() {
        return candidate;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    candidate = Path::new(output_dir).join(format!("{base}{suffix}-{timestamp}.{extension}"));
    candidate
}
