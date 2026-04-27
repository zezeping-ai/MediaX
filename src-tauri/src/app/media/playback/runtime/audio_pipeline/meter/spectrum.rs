use std::f32::consts::PI;

pub(super) const AUDIO_METER_BAND_COUNT: usize = 24;
pub(super) const AUDIO_SPECTRUM_WINDOW: usize = 1024;

pub(super) fn compute_spectrum(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.len() < 32 || sample_rate == 0 {
        return vec![0.0; AUDIO_METER_BAND_COUNT];
    }
    let window_len = samples.len().min(AUDIO_SPECTRUM_WINDOW);
    let slice = &samples[samples.len() - window_len..];
    let nyquist = (sample_rate as f32) * 0.5;
    let min_freq = 32.0f32;
    let max_freq = nyquist.mul_add(0.92, 0.0).min(16_000.0).max(min_freq * 2.0);
    let ratio = max_freq / min_freq;
    (0..AUDIO_METER_BAND_COUNT)
        .map(|index| {
            let band_t = if AUDIO_METER_BAND_COUNT <= 1 {
                0.0
            } else {
                index as f32 / (AUDIO_METER_BAND_COUNT - 1) as f32
            };
            let frequency = min_freq * ratio.powf(band_t);
            let omega = (2.0 * PI * frequency) / (sample_rate as f32);
            let mut real = 0.0f32;
            let mut imag = 0.0f32;
            for (sample_index, sample) in slice.iter().enumerate() {
                let pos = sample_index as f32 / (window_len.saturating_sub(1).max(1) as f32);
                let window = 0.5 - 0.5 * (2.0 * PI * pos).cos();
                let phase = omega * sample_index as f32;
                real += sample * window * phase.cos();
                imag -= sample * window * phase.sin();
            }
            let magnitude = (real.mul_add(real, imag * imag)).sqrt() / (window_len as f32);
            let db = 20.0 * (magnitude + 1e-6).log10();
            ((db + 54.0) / 54.0).clamp(0.0, 1.0)
        })
        .collect()
}
