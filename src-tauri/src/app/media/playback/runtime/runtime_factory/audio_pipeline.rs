use super::diagnostics::emit_audio_stream_diagnostics;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::playback::runtime::audio_pipeline::{
    build_audio_pipeline, AudioPipeline,
};
use crate::app::media::playback::runtime::emit::emit_debug;
use crate::app::media::state::{AudioControls, TimingControls};
use ffmpeg_next::media::Type;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn prepare_audio_pipeline(
    app: &AppHandle,
    video_ctx: &VideoDecodeContext,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
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
        timing_controls,
    )?;
    let debug_message = match audio_pipeline.as_ref() {
        Some(pipeline) => format!(
            "stream={} decoder_rate={}Hz decoder_channels={} decoder_fmt={:?} output=rodio/{} mode=unified-metered",
            pipeline.stream_index,
            pipeline.decoder.rate(),
            pipeline.decoder.channels(),
            pipeline.decoder.format(),
            pipeline.output_sample_format.debug_label(),
        ),
        None => "audio pipeline skipped (no audio stream)".to_string(),
    };
    emit_debug(app, "audio_pipeline_ready", debug_message);
    Ok(audio_pipeline)
}
