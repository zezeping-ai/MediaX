use crate::app::media::playback::dto::PlaybackMediaKind;
use crate::app::media::playback::decode_context::{
    cover_frame_from_image_bytes, load_deferred_audio_cover_frame, VideoDecodeContext,
};
use crate::app::media::playback::events::MediaMetadataPayload;
use crate::app::media::playback::render::renderer::RendererState;
use crate::app::media::playback::runtime::emit::{emit_debug, emit_metadata_payloads};
use crate::app::media::state::MediaState;
use std::path::Path;
use tauri::{AppHandle, Manager};

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

pub(super) fn spawn_deferred_audio_cover_load(
    app: &AppHandle,
    renderer: &RendererState,
    source: &str,
    stream_generation: u32,
    video_ctx: &VideoDecodeContext,
) {
    if video_ctx.media_kind != PlaybackMediaKind::Audio
        || video_ctx.cover_frame.is_some()
        || !video_ctx.has_cover_art
        || !Path::new(source).is_file()
    {
        return;
    }
    let app_handle = app.clone();
    let renderer = renderer.clone();
    let source = source.to_string();
    let title = video_ctx.title.clone();
    let artist = video_ctx.artist.clone();
    let album = video_ctx.album.clone();
    let lyrics = video_ctx.lyrics.clone();
    let duration_seconds = video_ctx.duration_seconds;
    let deferred_cover_bytes = video_ctx.deferred_cover_bytes.clone();
    std::thread::spawn(move || {
        let Some(frame) = load_cover_frame_if_current(
            &app_handle,
            &source,
            stream_generation,
            deferred_cover_bytes,
        ) else {
            return;
        };
        renderer.submit_frame(frame.clone());
        emit_metadata_payloads(
            &app_handle,
            MediaMetadataPayload {
                media_kind: PlaybackMediaKind::Audio,
                width: frame.width,
                height: frame.height,
                fps: 0.0,
                duration_seconds,
                title,
                artist,
                album,
                has_cover_art: true,
                lyrics,
            },
        );
        emit_debug(
            &app_handle,
            "metadata_ready",
            format!(
                "audio cover ready width={} height={} duration={duration_seconds:.3}s",
                frame.width, frame.height
            ),
        );
    });
}

fn load_cover_frame_if_current(
    app: &AppHandle,
    source: &str,
    stream_generation: u32,
    deferred_cover_bytes: Option<Vec<u8>>,
) -> Option<crate::app::media::playback::render::renderer::VideoFrame> {
    let media_state = app.state::<MediaState>();
    if !media_state
        .runtime
        .stream
        .is_generation_current(stream_generation)
    {
        return None;
    }
    let playback = media_state.session.playback.lock().ok()?;
    if playback.state().current_path.as_deref() != Some(source) {
        return None;
    }
    drop(playback);
    let frame = deferred_cover_bytes
        .as_deref()
        .and_then(|bytes| cover_frame_from_image_bytes(bytes).ok())
        .or_else(|| load_deferred_audio_cover_frame(source).ok().flatten())?;
    if !media_state
        .runtime
        .stream
        .is_generation_current(stream_generation)
    {
        return None;
    }
    let playback = media_state.session.playback.lock().ok()?;
    (playback.state().current_path.as_deref() == Some(source)).then_some(frame)
}
