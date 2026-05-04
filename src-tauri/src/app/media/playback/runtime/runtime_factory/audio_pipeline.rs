use super::diagnostics::emit_audio_stream_diagnostics;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::playback::runtime::audio_pipeline::{
    build_audio_pipeline, AudioPipeline, PlaybackHeadPrecision,
};
use crate::app::media::playback::runtime::emit::emit_debug;
use crate::app::media::state::AudioControls;
use ffmpeg_next::media::Type;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn prepare_audio_pipeline(
    app: &AppHandle,
    video_ctx: &VideoDecodeContext,
    audio_controls: &Arc<AudioControls>,
) -> Result<Option<AudioPipeline>, String> {
    let audio_stream_index = video_ctx
        .input_ctx
        .streams()
        .best(Type::Audio)
        .map(|stream| stream.index());
    emit_audio_stream_diagnostics(app, video_ctx, audio_stream_index);
    let audio_pipeline = build_audio_pipeline(
        app,
        &video_ctx.input_ctx,
        audio_stream_index,
        audio_controls,
    )?;
    let debug_message = match audio_pipeline.as_ref() {
        Some(pipeline) => {
            let playback_head_mode = match pipeline.output.playback_head_precision() {
                PlaybackHeadPrecision::Measured => "measured",
                PlaybackHeadPrecision::Estimated => "estimated",
            };
            format!(
                "stream={} decoder_rate={}Hz output_rate={}Hz decoder_channels={} decoder_fmt={:?} output={}/{} playback_head={} mode=unified-metered",
                pipeline.stream_index,
                pipeline.decoder.rate(),
                pipeline.output_sample_rate,
                pipeline.decoder.channels(),
                pipeline.decoder.format(),
                pipeline.output.backend_name(),
                pipeline.output_sample_format.debug_label(),
                playback_head_mode,
            )
        }
        None => "audio pipeline skipped (no audio stream)".to_string(),
    };
    eprintln!("mediax audio pipeline: {debug_message}");
    emit_debug(app, "audio_pipeline_ready", debug_message);
    Ok(audio_pipeline)
}
