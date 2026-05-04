use crate::app::media::playback::runtime::audio_pipeline::types::AudioOutputSampleFormat;
use ffmpeg_next::channel_layout::ChannelLayout;
use ffmpeg_next::software::resampling::context::Context as ResamplingContext;

pub(super) fn fallback_channel_layout(decoder: &ffmpeg_next::decoder::Audio) -> ChannelLayout {
    if decoder.channel_layout().is_empty() {
        ChannelLayout::default(decoder.channels().into())
    } else {
        decoder.channel_layout()
    }
}

pub(super) fn create_compatible_resampler(
    decoder: &ffmpeg_next::decoder::Audio,
    channel_layout: ChannelLayout,
    target_sample_rate: u32,
) -> Result<(ResamplingContext, AudioOutputSampleFormat), String> {
    let candidates = [
        AudioOutputSampleFormat::F32Packed,
        AudioOutputSampleFormat::I16Packed,
    ];
    let output_sample_rate = target_sample_rate.max(1);
    let mut errors = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        match ResamplingContext::get(
            decoder.format(),
            channel_layout,
            decoder.rate(),
            candidate.ffmpeg_sample_format(),
            channel_layout,
            output_sample_rate,
        ) {
            Ok(resampler) => return Ok((resampler, candidate)),
            Err(err) => errors.push(format!("{}: {err}", candidate.debug_label())),
        }
    }

    Err(format!(
        "decoder_fmt={:?} decoder_rate={}Hz output_rate={}Hz channels={} attempts=[{}]",
        decoder.format(),
        decoder.rate(),
        output_sample_rate,
        decoder.channels(),
        errors.join("; ")
    ))
}
