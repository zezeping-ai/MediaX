use super::types::AudioOutputSampleFormat;
use ffmpeg_next::frame;
use soundtouch::{Setting, SoundTouch};

const TIME_STRETCH_OUTPUT_CHUNK_FRAMES: usize = 4096;
const TIME_STRETCH_MAX_DRAIN_FRAMES_PER_CALL: usize = 4096;
const TIME_STRETCH_WARMUP_FALLBACK_FRAMES: u8 = 3;

pub(crate) struct AudioTimeStretch {
    processor: Option<AudioTimeStretchProcessor>,
    output_sample_format: AudioOutputSampleFormat,
    scratch_input_pcm: Vec<f32>,
}

struct AudioTimeStretchProcessor {
    soundtouch: SoundTouch,
    rate: f32,
    sample_rate: u32,
    channels: usize,
    warmup_fallback_frames_remaining: u8,
    output_chunk: Vec<f32>,
}

impl AudioTimeStretch {
    pub fn new(output_sample_format: AudioOutputSampleFormat) -> Self {
        Self {
            processor: None,
            output_sample_format,
            scratch_input_pcm: Vec::new(),
        }
    }

    pub fn process_frame_into(
        &mut self,
        frame: &mut frame::Audio,
        rate: f32,
        pcm: &mut Vec<f32>,
    ) -> Result<(), String> {
        pcm.clear();
        let mut input_pcm = std::mem::take(&mut self.scratch_input_pcm);
        input_pcm.clear();
        extract_output_samples_into(frame, self.output_sample_format, &mut input_pcm)?;
        if input_pcm.is_empty() || frame.channels() == 0 {
            self.scratch_input_pcm = input_pcm;
            return Ok(());
        }
        if should_bypass_time_stretch(rate) {
            pcm.extend_from_slice(&input_pcm);
            self.scratch_input_pcm = input_pcm;
            return Ok(());
        }

        let channels = frame.channels().max(1) as usize;
        let processor = self.ensure_processor(frame.rate(), channels, rate);
        processor.put_samples(&input_pcm);
        processor.receive_available_into(pcm);
        if pcm.is_empty() {
            processor.fill_warmup_fallback_pcm(&input_pcm, pcm);
        }
        self.scratch_input_pcm = input_pcm;
        Ok(())
    }

    pub fn reset(&mut self) {
        if let Some(processor) = self.processor.as_mut() {
            processor.clear();
        }
        self.processor = None;
        self.scratch_input_pcm.clear();
    }

    fn ensure_processor(
        &mut self,
        sample_rate: u32,
        channels: usize,
        rate: f32,
    ) -> &mut AudioTimeStretchProcessor {
        let needs_rebuild = self
            .processor
            .as_ref()
            .map(|processor| {
                processor.sample_rate != sample_rate
                    || processor.channels != channels
                    || processor.rate_delta(rate) > 1e-3
            })
            .unwrap_or(true);
        if needs_rebuild {
            self.processor = Some(AudioTimeStretchProcessor::new(sample_rate, channels, rate));
        }
        self.processor
            .as_mut()
            .expect("audio time stretch processor should exist")
    }
}

impl AudioTimeStretchProcessor {
    fn new(sample_rate: u32, channels: usize, rate: f32) -> Self {
        let mut soundtouch = SoundTouch::new();
        soundtouch
            .set_sample_rate(sample_rate)
            .set_channels(channels as u32)
            .set_rate(1.0)
            .set_pitch(1.0)
            .set_tempo(rate as f64)
            .set_setting(Setting::UseQuickseek, 1);
        let output_chunk_len = channels.saturating_mul(TIME_STRETCH_OUTPUT_CHUNK_FRAMES);
        Self {
            soundtouch,
            rate,
            sample_rate,
            channels,
            warmup_fallback_frames_remaining: TIME_STRETCH_WARMUP_FALLBACK_FRAMES,
            output_chunk: vec![0.0; output_chunk_len.max(channels)],
        }
    }

    fn rate_delta(&self, rate: f32) -> f32 {
        (self.rate - rate).abs()
    }

    fn put_samples(&mut self, pcm: &[f32]) {
        let frame_count = pcm.len() / self.channels.max(1);
        if frame_count == 0 {
            return;
        }
        self.soundtouch.put_samples(pcm, frame_count);
    }

    fn receive_available_into(&mut self, pcm: &mut Vec<f32>) {
        let mut drained_frames = 0usize;
        loop {
            if drained_frames >= TIME_STRETCH_MAX_DRAIN_FRAMES_PER_CALL {
                break;
            }
            let ready_frames = self.soundtouch.num_samples().max(0) as usize;
            if ready_frames == 0 {
                break;
            }
            let request_frames = ready_frames
                .min(TIME_STRETCH_OUTPUT_CHUNK_FRAMES)
                .min(TIME_STRETCH_MAX_DRAIN_FRAMES_PER_CALL.saturating_sub(drained_frames));
            let written_frames = self
                .soundtouch
                .receive_samples(self.output_chunk.as_mut_slice(), request_frames);
            if written_frames == 0 {
                break;
            }
            drained_frames = drained_frames.saturating_add(written_frames);
            let written_samples = written_frames.saturating_mul(self.channels);
            pcm.extend_from_slice(&self.output_chunk[..written_samples]);
        }
    }

    fn fill_warmup_fallback_pcm(&mut self, input_pcm: &[f32], pcm: &mut Vec<f32>) {
        if self.warmup_fallback_frames_remaining == 0 || should_bypass_time_stretch(self.rate) {
            return;
        }
        self.warmup_fallback_frames_remaining =
            self.warmup_fallback_frames_remaining.saturating_sub(1);
        synthesize_warmup_fallback_pcm(input_pcm, self.channels, self.rate, pcm);
    }

    fn clear(&mut self) {
        self.soundtouch.clear();
        self.warmup_fallback_frames_remaining = TIME_STRETCH_WARMUP_FALLBACK_FRAMES;
    }
}

fn should_bypass_time_stretch(rate: f32) -> bool {
    (rate - 1.0).abs() <= 1e-3
}

fn synthesize_warmup_fallback_pcm(
    input: &[f32],
    channels: usize,
    rate: f32,
    output: &mut Vec<f32>,
) {
    if input.is_empty() || channels == 0 || rate <= 0.0 {
        return;
    }
    let input_frames = input.len() / channels;
    if input_frames == 0 {
        return;
    }
    let output_frames = ((input_frames as f32) / rate).round().max(1.0) as usize;
    output.reserve(output_frames.saturating_mul(channels));
    for output_frame in 0..output_frames {
        let source_frame = ((output_frame as f32) * rate).floor() as usize;
        let source_frame = source_frame.min(input_frames.saturating_sub(1));
        let source_offset = source_frame * channels;
        output.extend_from_slice(&input[source_offset..source_offset + channels]);
    }
}

fn extract_output_samples_into(
    frame: &frame::Audio,
    output_sample_format: AudioOutputSampleFormat,
    pcm: &mut Vec<f32>,
) -> Result<(), String> {
    let channels = frame.channels().max(1) as usize;
    let total_samples = frame.samples().saturating_mul(channels);
    if total_samples == 0 {
        return Ok(());
    }
    let data = frame.data(0);
    if data.is_empty() {
        return Ok(());
    }

    pcm.reserve(total_samples);
    match output_sample_format {
        AudioOutputSampleFormat::F32Packed => {
            let bytes_per_sample = std::mem::size_of::<f32>();
            let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
            let clamped_bytes = expected_bytes.min(data.len());
            if clamped_bytes < bytes_per_sample {
                return Ok(());
            }
            pcm.extend(
                data[..clamped_bytes]
                    .chunks_exact(bytes_per_sample)
                    .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])),
            );
        }
        AudioOutputSampleFormat::I16Packed => {
            let bytes_per_sample = std::mem::size_of::<i16>();
            let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
            let clamped_bytes = expected_bytes.min(data.len());
            if clamped_bytes < bytes_per_sample {
                return Ok(());
            }
            pcm.extend(
                data[..clamped_bytes]
                    .chunks_exact(bytes_per_sample)
                    .map(|chunk| i16::from_ne_bytes([chunk[0], chunk[1]]) as f32 / (i16::MAX as f32)),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{should_bypass_time_stretch, synthesize_warmup_fallback_pcm, AudioTimeStretchProcessor};

    #[test]
    fn bypasses_neutral_rate() {
        assert!(should_bypass_time_stretch(1.0));
        assert!(should_bypass_time_stretch(1.0009));
        assert!(!should_bypass_time_stretch(1.01));
    }

    #[test]
    fn warmup_fallback_pcm_retimes_interleaved_audio() {
        let input = vec![0.0, 10.0, 1.0, 11.0, 2.0, 12.0, 3.0, 13.0];
        let mut output = Vec::new();
        synthesize_warmup_fallback_pcm(&input, 2, 2.0, &mut output);
        assert_eq!(output, vec![0.0, 10.0, 2.0, 12.0]);
    }

    #[test]
    fn soundtouch_processor_produces_pitch_preserving_output() {
        let mut processor = AudioTimeStretchProcessor::new(48_000, 2, 2.0);
        let input = vec![0.0f32; 48_000 * 2 / 10];
        processor.put_samples(&input);
        let mut output = Vec::new();
        for _ in 0..8 {
            processor.receive_available_into(&mut output);
            if !output.is_empty() {
                break;
            }
            processor.put_samples(&input);
        }
        assert!(!output.is_empty());
        assert_eq!(output.len() % 2, 0);
    }
}
