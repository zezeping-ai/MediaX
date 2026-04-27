use super::types::AudioOutputSampleFormat;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::filter;
use ffmpeg_next::format;
use ffmpeg_next::frame;

pub(crate) struct AudioTimeStretch {
    graph: Option<AudioTimeStretchGraph>,
    output_sample_format: AudioOutputSampleFormat,
}

struct AudioTimeStretchGraph {
    graph: filter::Graph,
    rate: f32,
    sample_rate: u32,
    sample_format: format::Sample,
    channel_layout: ChannelLayout,
}

impl AudioTimeStretch {
    pub fn new(output_sample_format: AudioOutputSampleFormat) -> Self {
        Self {
            graph: None,
            output_sample_format,
        }
    }

    pub fn process_frame(
        &mut self,
        frame: &frame::Audio,
        rate: f32,
    ) -> Result<Vec<f32>, String> {
        if should_bypass_time_stretch(rate) {
            return extract_output_samples(frame, self.output_sample_format);
        }
        let graph = self.ensure_graph(frame, rate)?;
        let mut filter_input = frame.clone();
        if filter_input.pts().is_none() {
            let best_effort_pts = filter_input.timestamp();
            filter_input.set_pts(best_effort_pts);
        }
        graph
            .graph
            .get("in")
            .ok_or_else(|| "audio time-stretch source missing".to_string())?
            .source()
            .add(&filter_input)
            .map_err(|err| format!("audio time-stretch source add failed: {err}"))?;
        self.drain_available_samples()
    }

    pub fn reset(&mut self) {
        self.graph = None;
    }

    fn ensure_graph(
        &mut self,
        frame: &frame::Audio,
        rate: f32,
    ) -> Result<&mut AudioTimeStretchGraph, String> {
        let sample_rate = frame.rate();
        let sample_format = frame.format();
        let channel_layout = frame.channel_layout();
        let needs_rebuild = self
            .graph
            .as_ref()
            .map(|graph| {
                (graph.rate - rate).abs() > 1e-3
                    || graph.sample_rate != sample_rate
                    || graph.sample_format != sample_format
                    || graph.channel_layout != channel_layout
            })
            .unwrap_or(true);
        if needs_rebuild {
            self.graph = Some(AudioTimeStretchGraph::new(
                sample_rate,
                sample_format,
                channel_layout,
                rate,
                self.output_sample_format,
            )?);
        }
        self.graph
            .as_mut()
            .ok_or_else(|| "audio time-stretch graph unavailable".to_string())
    }

    fn drain_available_samples(&mut self) -> Result<Vec<f32>, String> {
        let Some(graph) = self.graph.as_mut() else {
            return Ok(Vec::new());
        };
        let mut pcm = Vec::new();
        let mut filtered = frame::Audio::empty();
        loop {
            let result = graph
                .graph
                .get("out")
                .ok_or_else(|| "audio time-stretch sink missing".to_string())?
                .sink()
                .frame(&mut filtered);
            match result {
                Ok(()) => pcm.extend(extract_output_samples(&filtered, self.output_sample_format)?),
                Err(ffmpeg::Error::Other { errno }) if errno == ffmpeg::util::error::EAGAIN => break,
                Err(ffmpeg::Error::Eof) => break,
                Err(err) => return Err(format!("audio time-stretch drain failed: {err}")),
            }
        }
        Ok(pcm)
    }
}

impl AudioTimeStretchGraph {
    fn new(
        sample_rate: u32,
        sample_format: format::Sample,
        channel_layout: ChannelLayout,
        rate: f32,
        output_sample_format: AudioOutputSampleFormat,
    ) -> Result<Self, String> {
        let mut graph = filter::Graph::new();
        let args = format!(
            "time_base=1/{sample_rate}:sample_rate={sample_rate}:sample_fmt={}:channel_layout=0x{:x}",
            sample_format.name(),
            channel_layout.bits(),
        );

        graph
            .add(
                &filter::find("abuffer")
                    .ok_or_else(|| "ffmpeg abuffer filter unavailable".to_string())?,
                "in",
                &args,
            )
            .map_err(|err| format!("audio time-stretch create abuffer failed: {err}"))?;
        graph
            .add(
                &filter::find("abuffersink")
                    .ok_or_else(|| "ffmpeg abuffersink filter unavailable".to_string())?,
                "out",
                "",
            )
            .map_err(|err| format!("audio time-stretch create abuffersink failed: {err}"))?;

        {
            let mut out = graph
                .get("out")
                .ok_or_else(|| "audio time-stretch sink context missing".to_string())?;
            out.set_sample_format(output_sample_format.ffmpeg_sample_format());
            out.set_channel_layout(channel_layout);
            out.set_sample_rate(sample_rate);
        }

        graph
            .output("in", 0)
            .and_then(|parser| parser.input("out", 0))
            .and_then(|parser| parser.parse(&build_atempo_filter_spec(rate)))
            .map_err(|err| format!("audio time-stretch parse graph failed: {err}"))?;
        graph
            .validate()
            .map_err(|err| format!("audio time-stretch validate graph failed: {err}"))?;

        Ok(Self {
            graph,
            rate,
            sample_rate,
            sample_format,
            channel_layout,
        })
    }
}

fn should_bypass_time_stretch(rate: f32) -> bool {
    (rate - 1.0).abs() <= 1e-3
}

fn build_atempo_filter_spec(rate: f32) -> String {
    let mut remaining = rate.max(0.25) as f64;
    let mut stages = Vec::new();
    while remaining > 2.0 + 1e-6 {
        stages.push(2.0);
        remaining /= 2.0;
    }
    while remaining < 0.5 - 1e-6 {
        stages.push(0.5);
        remaining /= 0.5;
    }
    stages.push(remaining);
    stages
        .into_iter()
        .map(|stage| format!("atempo={stage:.6}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn extract_output_samples(
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

#[cfg(test)]
mod tests {
    use super::build_atempo_filter_spec;

    #[test]
    fn chains_large_tempo_values() {
        assert_eq!(build_atempo_filter_spec(3.0), "atempo=2.000000,atempo=1.500000");
    }

    #[test]
    fn chains_small_tempo_values() {
        assert_eq!(build_atempo_filter_spec(0.25), "atempo=0.500000,atempo=0.500000");
    }
}
