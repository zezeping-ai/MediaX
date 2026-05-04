use crate::app::media::playback::rate::PlaybackRate;
use crate::app::media::state::MediaState;
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format;
use tauri::{AppHandle, Manager};

use super::audio_pipeline::AudioPipeline;
use super::clock::PlaybackClock;

pub fn take_pending_seek_seconds(app: &AppHandle) -> Result<Option<f64>, String> {
    let media_state = app.state::<MediaState>();
    media_state
        .runtime
        .stream
        .take_pending_seek_seconds()
        .map_err(|err| err.to_string())
}

pub fn apply_seek_to_stream(
    input_ctx: &mut format::context::Input,
    decoder: Option<&mut ffmpeg::decoder::Video>,
    target_seconds: f64,
    playback_clock: &mut PlaybackClock,
    progress_position_seconds: &mut f64,
    audio_pipeline: Option<&mut AudioPipeline>,
) -> Result<(), String> {
    let clamped = target_seconds.max(0.0);
    // Some network streams (notably certain HLS playlists) are non-seekable and can return
    // EPERM/Operation not permitted. Startup resume commonly schedules a seek-to-zero, which
    // should behave as a no-op instead of failing playback.
    if clamped <= f64::EPSILON {
        playback_clock.reset_to(0.0);
        *progress_position_seconds = 0.0;
        return Ok(());
    }
    let ts = (clamped * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64;
    if let Err(err) = input_ctx.seek(ts, ..) {
        if is_non_seekable_error(err) {
            return Ok(());
        }
        return Err(format!("seek stream failed: {err}"));
    }
    if let Some(decoder) = decoder {
        decoder.flush();
    }
    if let Some(audio_state) = audio_pipeline {
        audio_state.decoder.flush();
        // Clearing queued sources pauses rodio playback. Resume immediately after seek.
        let current_rate = PlaybackRate::from_f64(playback_clock.playback_rate());
        audio_state.restart_after_discontinuity(current_rate, current_rate, false);
    }
    playback_clock.reset_to(clamped);
    *progress_position_seconds = clamped;
    Ok(())
}

fn is_non_seekable_error(err: ffmpeg::Error) -> bool {
    matches!(
        err,
        ffmpeg::Error::Other { errno }
            if errno == ffmpeg::util::error::EPERM
                || errno == ffmpeg::util::error::ESPIPE
                || errno == ffmpeg::util::error::ENOSYS
                || errno == ffmpeg::util::error::EOPNOTSUPP
    ) || err
        .to_string()
        .to_ascii_lowercase()
        .contains("operation not permitted")
}
