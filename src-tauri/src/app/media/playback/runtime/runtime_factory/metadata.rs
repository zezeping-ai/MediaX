use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::decode_context::VideoDecodeContext;
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::emit::{emit_debug, emit_metadata_payloads};
use tauri::AppHandle;

pub(super) fn emit_decoder_ready(app: &AppHandle, video_ctx: &VideoDecodeContext) {
    let message = match video_ctx.decoder.as_ref() {
        Some(decoder) => format!(
            "decoder={:?} hw_active={} hw_backend={} input={}x{} fps={:.3} duration={:.3}s output={}x{}",
            decoder.id(),
            video_ctx.hw_decode_active,
            video_ctx.hw_decode_backend.as_deref().unwrap_or("software"),
            decoder.width(),
            decoder.height(),
            video_ctx.fps_value,
            video_ctx.duration_seconds,
            video_ctx.output_width,
            video_ctx.output_height,
        ),
        None => format!(
            "audio_only hw_active={} duration={:.3}s cover={} lyrics={}",
            video_ctx.hw_decode_active,
            video_ctx.duration_seconds,
            video_ctx.has_cover_art,
            video_ctx.lyrics.len()
        ),
    };
    emit_debug(app, "decoder_ready", message);
}

pub(super) fn emit_runtime_metadata(app: &AppHandle, video_ctx: &VideoDecodeContext) {
    emit_metadata_payloads(
        app,
        MediaMetadataPayload {
            media_kind: video_ctx.media_kind,
            width: video_ctx.output_width,
            height: video_ctx.output_height,
            fps: video_ctx.fps_value,
            duration_seconds: video_ctx.duration_seconds,
            title: video_ctx.title.clone(),
            artist: video_ctx.artist.clone(),
            album: video_ctx.album.clone(),
            has_cover_art: video_ctx.has_cover_art,
            lyrics: video_ctx.lyrics.clone(),
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
}

pub(super) fn prime_audio_poster_frame(renderer: &RendererState, video_ctx: &VideoDecodeContext) {
    if video_ctx.media_kind != PlaybackMediaKind::Audio {
        return;
    }
    if let Some(frame) = video_ctx.cover_frame.as_ref() {
        renderer.submit_frame(frame.clone());
    }
}
