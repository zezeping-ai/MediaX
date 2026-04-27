use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::state::TimingControls;
use ffmpeg_next::frame;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn sync_audio_clock(
    decoded: &frame::Audio,
    time_base: ffmpeg_next::Rational,
    timing_controls: &Arc<TimingControls>,
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
                timing_controls.playback_rate().max(0.25) as f64,
            ));
        }
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
