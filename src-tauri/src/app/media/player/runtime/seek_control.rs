use crate::app::media::player::state::MediaState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use tauri::{AppHandle, Manager};

use super::AudioPipeline;
use super::clock::PlaybackClock;

pub fn take_pending_seek_seconds(app: &AppHandle) -> Result<Option<f64>, String> {
    let media_state = app.state::<MediaState>();
    let mut guard = media_state
        .pending_seek_seconds
        .lock()
        .map_err(|_| "pending seek state poisoned".to_string())?;
    Ok(guard.take())
}

pub fn apply_seek_to_stream(
    input_ctx: &mut format::context::Input,
    decoder: &mut ffmpeg::decoder::Video,
    target_seconds: f64,
    playback_clock: &mut PlaybackClock,
    current_position_seconds: &mut f64,
    audio_pipeline: Option<&mut AudioPipeline>,
) -> Result<(), String> {
    let clamped = target_seconds.max(0.0);
    let ts = (clamped * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
    input_ctx
        .seek(ts, ..)
        .map_err(|err| format!("seek stream failed: {err}"))?;
    decoder.flush();
    if let Some(audio_state) = audio_pipeline {
        audio_state.decoder.flush();
        audio_state.output.player.clear();
        // rodio::Player::clear() also pauses playback. Ensure audio resumes after seek.
        audio_state.output.player.play();
    }
    playback_clock.reset_to(clamped);
    *current_position_seconds = clamped;
    Ok(())
}
