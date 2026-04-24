use crate::app::media::library::MediaLibraryService;
use crate::app::media::playback::MediaPlaybackService;
use crate::app::media::types::MediaSnapshot;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::codec;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};

const MEDIA_STATE_EVENT: &str = "media://state";
const MEDIA_FRAME_EVENT: &str = "media://frame";
const MEDIA_METADATA_EVENT: &str = "media://metadata";
const MEDIA_ERROR_EVENT: &str = "media://error";

#[derive(Clone, Serialize)]
struct MediaFramePayload {
    width: u32,
    height: u32,
    position_seconds: f64,
    rgba: Vec<u8>,
}

#[derive(Clone, Serialize)]
struct MediaMetadataPayload {
    width: u32,
    height: u32,
    fps: f64,
    duration_seconds: f64,
}

#[derive(Clone, Serialize)]
struct MediaErrorPayload {
    code: &'static str,
    message: String,
}

#[derive(Default)]
pub struct MediaState {
    pub library: Mutex<MediaLibraryService>,
    pub playback: Mutex<MediaPlaybackService>,
    pub stream_stop_flag: Mutex<Option<Arc<AtomicBool>>>,
}

#[tauri::command]
pub fn media_get_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    snapshot_from_state(&state)
}

#[tauri::command]
pub fn media_set_library_roots(
    app: AppHandle,
    state: State<'_, MediaState>,
    roots: Vec<String>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.set_roots_and_scan(roots);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_rescan_library(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.rescan();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_open(
    app: AppHandle,
    state: State<'_, MediaState>,
    path: String,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.open(path.clone());
    }
    {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, 0.0);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_play(app: AppHandle, state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let current_path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.play();
        playback.state().current_path
    };
    if let Some(source) = current_path {
        start_decode_stream(&app, &state, source)?;
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_pause(
    app: AppHandle,
    state: State<'_, MediaState>,
) -> Result<MediaSnapshot, String> {
    stop_decode_stream(&state)?;
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.pause();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_stop(app: AppHandle, state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    stop_decode_stream(&state)?;
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.stop();
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_seek(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
) -> Result<MediaSnapshot, String> {
    let path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.seek(position_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_set_rate(
    app: AppHandle,
    state: State<'_, MediaState>,
    playback_rate: f64,
) -> Result<MediaSnapshot, String> {
    {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.set_rate(playback_rate);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_sync_position(
    app: AppHandle,
    state: State<'_, MediaState>,
    position_seconds: f64,
    duration_seconds: f64,
) -> Result<MediaSnapshot, String> {
    let path = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.sync_position(position_seconds, duration_seconds);
        playback.state().current_path
    };
    if let Some(path) = path {
        let mut library = state
            .library
            .lock()
            .map_err(|_| "media library state poisoned".to_string())?;
        library.mark_playback_progress(&path, position_seconds);
    }
    emit_snapshot(&app, &state)
}

#[tauri::command]
pub fn media_start_stream(
    app: AppHandle,
    state: State<'_, MediaState>,
    source: String,
) -> Result<(), String> {
    start_decode_stream(&app, &state, source)
}

#[tauri::command]
pub fn media_stop_stream(state: State<'_, MediaState>) -> Result<(), String> {
    stop_decode_stream(&state)
}

fn emit_snapshot(app: &AppHandle, state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let snapshot = snapshot_from_state(state)?;
    app.emit(MEDIA_STATE_EVENT, &snapshot)
        .map_err(|err| format!("emit media state failed: {err}"))?;
    Ok(snapshot)
}

fn stop_decode_stream(state: &State<'_, MediaState>) -> Result<(), String> {
    let mut guard = state
        .stream_stop_flag
        .lock()
        .map_err(|_| "stream state poisoned".to_string())?;
    if let Some(flag) = guard.take() {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

fn start_decode_stream(app: &AppHandle, state: &State<'_, MediaState>, source: String) -> Result<(), String> {
    stop_decode_stream(state)?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = state
            .stream_stop_flag
            .lock()
            .map_err(|_| "stream state poisoned".to_string())?;
        *guard = Some(stop_flag.clone());
    }

    let app_handle = app.clone();
    thread::spawn(move || {
        if let Err(err) = decode_and_emit_stream(&app_handle, &source, &stop_flag) {
            let _ = app_handle.emit(
                MEDIA_ERROR_EVENT,
                MediaErrorPayload {
                    code: "DECODE_FAILED",
                    message: err,
                },
            );
        }
    });
    Ok(())
}

fn decode_and_emit_stream(
    app: &AppHandle,
    source: &str,
    stop_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    ffmpeg::init().map_err(|err| format!("ffmpeg init failed: {err}"))?;
    let mut input_ctx = format::input(source).map_err(|err| format!("open media failed: {err}"))?;

    let input_stream = input_ctx
        .streams()
        .best(Type::Video)
        .ok_or_else(|| "no video stream found".to_string())?;
    let video_stream_index = input_stream.index();
    let stream_time_base = input_stream.time_base();
    let stream_duration = input_stream.duration();
    let fps = input_stream.avg_frame_rate();
    let fps_value = if fps.denominator() != 0 {
        f64::from(fps.numerator()) / f64::from(fps.denominator())
    } else {
        0.0
    };
    let duration_seconds = if stream_duration > 0 {
        (stream_duration as f64) * f64::from(stream_time_base)
    } else {
        0.0
    };

    let codec_context = codec::context::Context::from_parameters(input_stream.parameters())
        .map_err(|err| format!("decoder context failed: {err}"))?;
    let mut decoder = codec_context
        .decoder()
        .video()
        .map_err(|err| format!("video decoder create failed: {err}"))?;

    let mut scaler = ScalingContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        format::pixel::Pixel::RGBA,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )
    .map_err(|err| format!("scaler create failed: {err}"))?;

    let _ = app.emit(
        MEDIA_METADATA_EVENT,
        MediaMetadataPayload {
            width: decoder.width(),
            height: decoder.height(),
            fps: fps_value,
            duration_seconds,
        },
    );

    for (stream, packet) in input_ctx.packets() {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        if stream.index() != video_stream_index {
            continue;
        }

        decoder
            .send_packet(&packet)
            .map_err(|err| format!("send packet failed: {err}"))?;
        drain_frames(app, &mut decoder, &mut scaler, stream_time_base, stop_flag)?;
    }

    decoder.send_eof().map_err(|err| format!("send eof failed: {err}"))?;
    drain_frames(app, &mut decoder, &mut scaler, stream_time_base, stop_flag)?;
    Ok(())
}

fn drain_frames(
    app: &AppHandle,
    decoder: &mut ffmpeg::decoder::Video,
    scaler: &mut ScalingContext,
    time_base: ffmpeg::Rational,
    stop_flag: &Arc<AtomicBool>,
) -> Result<(), String> {
    let mut decoded = frame::Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut rgba_frame = frame::Video::empty();
        scaler
            .run(&decoded, &mut rgba_frame)
            .map_err(|err| format!("scale frame failed: {err}"))?;

        let width = rgba_frame.width() as usize;
        let height = rgba_frame.height() as usize;
        let stride = rgba_frame.stride(0);
        let data = rgba_frame.data(0);
        let row_bytes = width * 4;
        let mut rgba = Vec::with_capacity(row_bytes * height);
        for y in 0..height {
            let start = y * stride;
            let end = start + row_bytes;
            rgba.extend_from_slice(&data[start..end]);
        }

        let pts = decoded.pts().unwrap_or(0);
        let position_seconds = (pts as f64) * f64::from(time_base);
        let _ = app.emit(
            MEDIA_FRAME_EVENT,
            MediaFramePayload {
                width: rgba_frame.width(),
                height: rgba_frame.height(),
                position_seconds,
                rgba,
            },
        );
    }
    Ok(())
}

fn snapshot_from_state(state: &State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    let library = state
        .library
        .lock()
        .map_err(|_| "media library state poisoned".to_string())?
        .state();
    let playback = {
        let mut playback = state
            .playback
            .lock()
            .map_err(|_| "playback state poisoned".to_string())?;
        playback.state()
    };
    Ok(MediaSnapshot { playback, library })
}
