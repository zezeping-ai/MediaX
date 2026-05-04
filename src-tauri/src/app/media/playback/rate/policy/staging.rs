use super::constants::{
    OUTPUT_STAGING_FRAMES_AUDIO_ONLY_DEFAULT, OUTPUT_STAGING_FRAMES_AUDIO_ONLY_FAST,
    OUTPUT_STAGING_FRAMES_AUDIO_ONLY_SLOW, OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_AUDIO_ONLY,
    OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_REALTIME, OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_VIDEO,
    OUTPUT_STAGING_FRAMES_SEEK_REFILL_AUDIO_ONLY, OUTPUT_STAGING_FRAMES_SEEK_REFILL_VIDEO,
    OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_LOCAL, OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_NETWORK,
    OUTPUT_STAGING_FRAMES_SEEK_SETTLE_VIDEO, OUTPUT_STAGING_FRAMES_VIDEO_DEFAULT,
    OUTPUT_STAGING_FRAMES_VIDEO_FAST, OUTPUT_STAGING_FRAMES_VIDEO_SLOW,
};
use super::super::value::PlaybackRate;

pub fn output_staging_frames(
    playback_rate: PlaybackRate,
    has_video_stream: bool,
    _is_network_source: bool,
) -> usize {
    let playback_rate = playback_rate.as_f32();
    if has_video_stream {
        if playback_rate >= 1.25 {
            OUTPUT_STAGING_FRAMES_VIDEO_FAST
        } else if playback_rate <= 0.75 {
            OUTPUT_STAGING_FRAMES_VIDEO_SLOW
        } else {
            OUTPUT_STAGING_FRAMES_VIDEO_DEFAULT
        }
    } else if playback_rate >= 1.25 {
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_FAST
    } else if playback_rate <= 0.75 {
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_SLOW
    } else {
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_DEFAULT
    }
}

pub fn rate_switch_cover_output_staging_frames(
    has_video_stream: bool,
    is_realtime_source: bool,
) -> usize {
    if is_realtime_source {
        return OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_REALTIME;
    }
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_VIDEO
    } else {
        OUTPUT_STAGING_FRAMES_RATE_SWITCH_COVER_AUDIO_ONLY
    }
}

pub fn seek_refill_output_staging_frames(has_video_stream: bool) -> usize {
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_SEEK_REFILL_VIDEO
    } else {
        OUTPUT_STAGING_FRAMES_SEEK_REFILL_AUDIO_ONLY
    }
}

pub fn seek_settle_output_staging_frames(has_video_stream: bool, is_network_source: bool) -> usize {
    if has_video_stream {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_VIDEO
    } else if is_network_source {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_NETWORK
    } else {
        OUTPUT_STAGING_FRAMES_SEEK_SETTLE_AUDIO_ONLY_LOCAL
    }
}

