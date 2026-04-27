mod resampler;

use super::output::AudioOutput;
use super::types::AudioPipeline;
use crate::app::media::state::AudioControls;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use std::sync::Arc;
use tauri::AppHandle;

use self::resampler::{create_compatible_resampler, fallback_channel_layout};

pub(crate) fn build_audio_pipeline(
    app: &AppHandle,
    input_ctx: &format::context::Input,
    audio_stream_index: Option<usize>,
    audio_controls: &Arc<AudioControls>,
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
    let channel_layout = fallback_channel_layout(&decoder);
    let (resampler, output_sample_format) =
        create_compatible_resampler(&decoder, channel_layout)
            .map_err(|err| format!("audio resampler create failed: {err}"))?;
    let output = AudioOutput::new(app, audio_controls.clone())?;
    Ok(Some(AudioPipeline {
        stream_index,
        decoder,
        time_base: input_stream.time_base(),
        resampler,
        output_sample_format,
        time_stretch: super::time_stretch::AudioTimeStretch::new(output_sample_format),
        output,
        stats: Default::default(),
        discontinuity_fade_in_frames_remaining: 0,
        output_staging_channels: 0,
        output_staging_samples: Vec::new(),
        recent_output_tail_channels: 0,
        recent_output_tail_samples: Vec::new(),
        discontinuity_crossfade_tail_channels: 0,
        discontinuity_crossfade_tail_samples: Vec::new(),
    }))
}
