use crate::app::media::state::MediaState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use tauri::{AppHandle, Manager};

use super::audio_pipeline::AudioPipeline;
use super::clock::PlaybackClock;

pub fn take_pending_seek_seconds(app: &AppHandle) -> Result<Option<f64>, String> {
    let media_state = app.state::<MediaState>();
    media_state
        .stream
        .take_pending_seek_seconds()
        .map_err(|err| err.to_string())
}

pub fn apply_seek_to_stream(
    input_ctx: &mut format::context::Input,
    decoder: Option<&mut ffmpeg::decoder::Video>,
    target_seconds: f64,
    playback_clock: &mut PlaybackClock,
    current_position_seconds: &mut f64,
    audio_pipeline: Option<&mut AudioPipeline>,
) -> Result<(), String> {
    let clamped = target_seconds.max(0.0);
    // Some network streams (notably certain HLS playlists) are non-seekable and can return
    // EPERM/Operation not permitted. Startup resume commonly schedules a seek-to-zero, which
    // should behave as a no-op instead of failing playback.
    if clamped <= f64::EPSILON {
        playback_clock.reset_to(0.0);
        *current_position_seconds = 0.0;
        return Ok(());
    }
    let ts = (clamped * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
    input_ctx
        .seek(ts, ..)
        .map_err(|err| format!("seek stream failed: {err}"))?;
    if let Some(decoder) = decoder {
        decoder.flush();
    }
    if let Some(audio_state) = audio_pipeline {
        audio_state.decoder.flush();
        // Clearing queued sources pauses rodio playback. Resume immediately after seek.
        audio_state.output.clear_queue();
    }
    playback_clock.reset_to(clamped);
    *current_position_seconds = clamped;
    Ok(())
}
