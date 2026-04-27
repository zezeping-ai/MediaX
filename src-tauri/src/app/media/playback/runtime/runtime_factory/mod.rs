mod audio_pipeline;
mod diagnostics;
mod media_state;
mod metadata;
mod playback_snapshot;
mod source_flags;

use super::audio::clamp_playback_rate;
use super::decode_params::{DecodeDependencies, DecodeRequest};
use super::emit::emit_debug;
use super::session::DecodeLoopState;
use super::DecodeRuntime;
use crate::app::media::playback::decode_context::open_video_decode_context;
use crate::app::media::state::MediaState;
use tauri::Manager;

use self::audio_pipeline::prepare_audio_pipeline;
use self::diagnostics::emit_video_stream_diagnostics;
use self::media_state::{resolve_quality_mode, sync_media_kind};
use self::metadata::{emit_decoder_ready, emit_runtime_metadata, prime_audio_poster_frame};
use self::playback_snapshot::update_hw_decode_snapshot;
use self::source_flags::should_tail_eof_for_source;

pub(super) fn create_decode_runtime(
    dependencies: &DecodeDependencies<'_>,
    request: &DecodeRequest<'_>,
) -> Result<DecodeRuntime, String> {
    let should_tail_eof = should_tail_eof_for_source(request.source);
    let media_state = dependencies.app.state::<MediaState>();
    let quality_mode = resolve_quality_mode(&media_state)?;
    emit_debug(
        dependencies.app,
        "open",
        format!(
            "source={} hw_mode={:?} quality_mode={quality_mode:?}",
            request.source, request.hw_mode_override,
        ),
    );
    let video_ctx = open_video_decode_context(
        request.source,
        request.hw_mode_override,
        quality_mode,
        request.software_fallback_reason,
        request.force_audio_only,
    )?;
    sync_media_kind(&media_state, video_ctx.media_kind)?;
    emit_decoder_ready(dependencies.app, &video_ctx);
    emit_debug(
        dependencies.app,
        "hw_decode_decision",
        video_ctx.hw_decode_decision.clone(),
    );
    emit_video_stream_diagnostics(dependencies.app, &video_ctx);
    update_hw_decode_snapshot(dependencies.app, &video_ctx)?;
    let audio_pipeline = prepare_audio_pipeline(
        dependencies.app,
        &video_ctx,
        dependencies.audio_controls,
    )?;
    emit_runtime_metadata(dependencies.app, &video_ctx);
    dependencies.renderer.reset_timeline(
        0.0,
        clamp_playback_rate(dependencies.timing_controls.playback_rate()) as f64,
    );
    prime_audio_poster_frame(dependencies.renderer, &video_ctx);
    emit_debug(dependencies.app, "running", "decode loop running");
    Ok(DecodeRuntime {
        loop_state: DecodeLoopState::new(video_ctx.fps_value, dependencies.timing_controls.clone()),
        video_ctx,
        scaler: None,
        audio_pipeline,
        should_tail_eof,
    })
}
