use super::types::{AudioOutputSampleFormat, AudioPipeline};
use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;
use std::sync::Arc;
use tauri::AppHandle;

pub(crate) fn build_audio_pipeline(
    app: &AppHandle,
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
    let (resampler, output_sample_format) =
        create_compatible_resampler(&decoder, channel_layout)
            .map_err(|err| format!("audio resampler create failed: {err}"))?;
    let output = super::types::AudioOutput::new(
        app,
        audio_controls.clone(),
        timing_controls.clone(),
    )?;
    Ok(Some(AudioPipeline {
        stream_index,
        decoder,
        time_base: input_stream.time_base(),
        resampler,
        output_sample_format,
        output,
        stats: Default::default(),
    }))
}

fn create_compatible_resampler(
    decoder: &ffmpeg_next::decoder::Audio,
    channel_layout: ChannelLayout,
) -> Result<(ResamplingContext, AudioOutputSampleFormat), String> {
    let candidates = [
        AudioOutputSampleFormat::F32Packed,
        AudioOutputSampleFormat::I16Packed,
    ];
    let mut errors = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        match ResamplingContext::get(
            decoder.format(),
            channel_layout,
            decoder.rate(),
            candidate.ffmpeg_sample_format(),
            channel_layout,
            decoder.rate(),
        ) {
            Ok(resampler) => return Ok((resampler, candidate)),
            Err(err) => errors.push(format!("{}: {err}", candidate.debug_label())),
        }
    }

    Err(format!(
        "decoder_fmt={:?} rate={}Hz channels={} attempts=[{}]",
        decoder.format(),
        decoder.rate(),
        decoder.channels(),
        errors.join("; ")
    ))
}
