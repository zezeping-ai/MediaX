use super::super::types::AudioOutputSampleFormat;
use ffmpeg_next::frame;

pub(super) fn extract_output_samples(
    frame: &frame::Audio,
    output_sample_format: AudioOutputSampleFormat,
) -> Result<Vec<f32>, String> {
    let channels = frame.channels().max(1) as usize;
    let total_samples = frame.samples().saturating_mul(channels);
    if total_samples == 0 {
        return Ok(Vec::new());
    }
    let data = frame.data(0);
    if data.is_empty() {
        return Ok(Vec::new());
    }

    match output_sample_format {
        AudioOutputSampleFormat::F32Packed => {
            let bytes_per_sample = std::mem::size_of::<f32>();
            let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
            let clamped_bytes = expected_bytes.min(data.len());
            if clamped_bytes < bytes_per_sample {
                return Ok(Vec::new());
            }
            Ok(data[..clamped_bytes]
                .chunks_exact(bytes_per_sample)
                .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect())
        }
        AudioOutputSampleFormat::I16Packed => {
            let bytes_per_sample = std::mem::size_of::<i16>();
            let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
            let clamped_bytes = expected_bytes.min(data.len());
            if clamped_bytes < bytes_per_sample {
                return Ok(Vec::new());
            }
            Ok(data[..clamped_bytes]
                .chunks_exact(bytes_per_sample)
                .map(|chunk| i16::from_ne_bytes([chunk[0], chunk[1]]) as f32 / (i16::MAX as f32))
                .collect())
        }
    }
}
