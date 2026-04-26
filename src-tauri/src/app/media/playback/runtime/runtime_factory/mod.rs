mod diagnostics;
mod playback_snapshot;
mod source_flags;

use super::audio::clamp_playback_rate;
use super::audio_pipeline::build_audio_pipeline;
use super::emit::{emit_debug, emit_metadata_payloads};
use super::session::DecodeLoopState;
use super::DecodeRuntime;
use crate::app::media::error::MediaError;
use crate::app::media::model::HardwareDecodeMode;
use crate::app::media::playback::decode_context::open_video_decode_context;
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::state::{AudioControls, MediaState, TimingControls};
use ffmpeg_next::media::Type;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use self::diagnostics::{emit_audio_stream_diagnostics, emit_video_stream_diagnostics};
use self::playback_snapshot::update_hw_decode_snapshot;
use self::source_flags::should_tail_eof_for_source;

pub(super) fn create_decode_runtime(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    audio_controls: &Arc<AudioControls>,
    timing_controls: &Arc<TimingControls>,
    hw_mode_override: HardwareDecodeMode,
    software_fallback_reason: Option<&str>,
) -> Result<DecodeRuntime, String> {
    let should_tail_eof = should_tail_eof_for_source(source);
    let media_state = app.state::<MediaState>();
    let quality_mode = {
        let playback = media_state
            .playback
            .lock()
            .map_err(|_| MediaError::state_poisoned_lock("playback state").to_string())?;
        playback.quality_mode()
    };
    emit_debug(
        app,
        "open",
        format!(
            "source={} hw_mode={hw_mode_override:?} quality_mode={quality_mode:?}",
            source
        ),
    );
    let video_ctx = open_video_decode_context(
        source,
        hw_mode_override,
        quality_mode,
        software_fallback_reason,
    )?;
    emit_debug(
        app,
        "decoder_ready",
        format!(
            "decoder={:?} hw_active={} hw_backend={} input={}x{} fps={:.3} duration={:.3}s output={}x{}",
            video_ctx.decoder.id(),
            video_ctx.hw_decode_active,
            video_ctx.hw_decode_backend.as_deref().unwrap_or("software"),
            video_ctx.decoder.width(),
            video_ctx.decoder.height(),
            video_ctx.fps_value,
            video_ctx.duration_seconds,
            video_ctx.output_width,
            video_ctx.output_height,
        ),
    );
    emit_debug(app, "hw_decode_decision", video_ctx.hw_decode_decision.clone());
    emit_video_stream_diagnostics(app, &video_ctx);
    update_hw_decode_snapshot(app, &video_ctx)?;
    let audio_stream_index = video_ctx
        .input_ctx
        .streams()
        .best(Type::Audio)
        .map(|stream| stream.index());
    emit_audio_stream_diagnostics(app, &video_ctx, audio_stream_index);
    let audio_pipeline = build_audio_pipeline(
        &video_ctx.input_ctx,
        audio_stream_index,
        audio_controls,
        timing_controls,
    )?;
    emit_debug(
        app,
        "audio_pipeline_ready",
        match audio_pipeline.as_ref() {
            Some(pipeline) => format!(
                "stream={} decoder_rate={}Hz decoder_channels={} decoder_fmt={:?} output=rodio/i16-packed",
                pipeline.stream_index,
                pipeline.decoder.rate(),
                pipeline.decoder.channels(),
                pipeline.decoder.format(),
            ),
            None => "audio pipeline skipped (no audio stream)".to_string(),
        },
    );
    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            width: video_ctx.output_width,
            height: video_ctx.output_height,
            fps: video_ctx.fps_value,
            duration_seconds: video_ctx.duration_seconds,
        },
    );
    emit_debug(
        app,
        "metadata_ready",
        format!(
            "container={} width={} height={} fps={:.3} duration={:.3}s",
            video_ctx.input_ctx.format().name(),
            video_ctx.output_width,
            video_ctx.output_height,
            video_ctx.fps_value,
            video_ctx.duration_seconds,
        ),
    );
    renderer.reset_timeline(0.0, clamp_playback_rate(timing_controls.playback_rate()) as f64);
    emit_debug(app, "running", "decode loop running");
    Ok(DecodeRuntime {
        loop_state: DecodeLoopState::new(video_ctx.fps_value, timing_controls.clone()),
        video_ctx,
        scaler: None,
        audio_pipeline,
        should_tail_eof,
    })
}
