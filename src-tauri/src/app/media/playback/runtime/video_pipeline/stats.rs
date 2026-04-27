pub(crate) fn percentile_from_sorted(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let normalized = percentile.clamp(0.0, 100.0) / 100.0;
    let position = normalized * ((sorted.len() - 1) as f64);
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let weight = position - (lower as f64);
    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
}
