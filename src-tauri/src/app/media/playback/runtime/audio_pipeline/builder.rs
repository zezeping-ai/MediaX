use super::types::AudioPipeline;
use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::format::sample::Type as SampleType;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use std::sync::Arc;

pub(crate) fn build_audio_pipeline(
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
) -> Result<Option<AudioPipeline>, String> {
    let Some(stream_index) = audio_stream_index else {
        return Ok(None);
    };
    let input_stream = input_ctx
        .streams()
        .find(|stream| stream.index() == stream_index)
        .ok_or_else(|| "audio stream index not found".to_string())?;
    let audio_context = codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|err| format!("audio decoder context failed: {err}"))?;
    let decoder = audio_context
        .decoder()
        .audio()
        .map_err(|err| format!("audio decoder create failed: {err}"))?;
    let channel_layout = if decoder.channel_layout().is_empty() {
        ChannelLayout::default(decoder.channels().into())
    } else {
        decoder.channel_layout()
    };
    let resampler = ResamplingContext::get(
        decoder.format(),
        channel_layout,
        decoder.rate(),
        ffmpeg_next::format::Sample::I16(SampleType::Packed),
        channel_layout,
        decoder.rate(),
    )
    .map_err(|err| format!("audio resampler create failed: {err}"))?;
    let output = super::types::AudioOutput::new(audio_controls.clone(), timing_controls.clone())?;
    Ok(Some(AudioPipeline {
        stream_index,
        decoder,
        time_base: input_stream.time_base(),
        resampler,
        output,
        stats: Default::default(),
    }))
}
