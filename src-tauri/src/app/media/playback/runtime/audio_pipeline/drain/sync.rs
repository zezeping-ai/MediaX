use crate::app::media::playback::rate::PlaybackRate;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::emit_debug;
use ffmpeg_next::frame;
use tauri::AppHandle;

pub(super) fn sync_audio_clock(
    decoded: &frame::Audio,
    time_base: ffmpeg_next::Rational,
    playback_rate: PlaybackRate,
    audio_clock: &mut Option<AudioClock>,
    active_seek_target_seconds: &mut Option<f64>,
) {
    if let Some(seconds) = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), time_base)
        .filter(|value| value.is_finite() && *value >= 0.0)
    {
        if let Some(target) = *active_seek_target_seconds {
            if seconds + 0.03 < target {
                return;
            }
            *active_seek_target_seconds = None;
        }
        if audio_clock.is_none() {
            *audio_clock = Some(AudioClock::new(
                seconds,
                playback_rate.as_f64(),
            ));
        }
    }
}

pub(super) fn sync_audio_clock_to_output_head(
    frame_start_seconds: Option<f64>,
    output_samples: usize,
    channels: usize,
    sample_rate: u32,
    playback_rate: PlaybackRate,
    queued_wall_seconds: f64,
    audio_clock: &mut Option<AudioClock>,
) {
    let Some(frame_start_seconds) =
        frame_start_seconds.filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return;
    };
    if channels == 0 || sample_rate == 0 || output_samples == 0 {
        return;
    }
    let output_frames = output_samples / channels;
    if output_frames == 0 {
        return;
    }
    let playback_rate_f64 = playback_rate.as_f64().max(0.25);
    let output_wall_seconds = output_frames as f64 / sample_rate as f64;
    let enqueued_media_end_seconds = frame_start_seconds + output_wall_seconds * playback_rate_f64;
    let playback_head_seconds =
        (enqueued_media_end_seconds - queued_wall_seconds.max(0.0) * playback_rate_f64).max(0.0);
    match audio_clock.as_mut() {
        Some(clock) => clock.rebase_position(playback_head_seconds, playback_rate_f64),
        None => *audio_clock = Some(AudioClock::new(playback_head_seconds, playback_rate_f64)),
    }
}

pub(super) fn should_drop_pre_seek_audio_frame(
    app: &AppHandle,
    decoded: &frame::Audio,
    time_base: ffmpeg_next::Rational,
    active_seek_target_seconds: &Option<f64>,
) -> bool {
    let Some(target) = *active_seek_target_seconds else {
        return false;
    };
    let Some(seconds) = timestamp_to_seconds(decoded.timestamp(), decoded.pts(), time_base)
        .filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return false;
    };
    if seconds + 0.03 < target {
        emit_debug(
            app,
            "audio_seek_drop",
            format!(
                "drop stale audio frame pts={seconds:.3}s target={target:.3}s delta_ms={:.3}",
                (target - seconds) * 1000.0
            ),
        );
        return true;
    }
    false
}
