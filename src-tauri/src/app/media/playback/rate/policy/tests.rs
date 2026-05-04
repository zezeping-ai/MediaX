use super::super::value::PlaybackRate;
use super::{
    audio_queue_depth_limit, audio_queue_prefill_target, audio_queue_refill_floor_seconds,
    audio_queue_seconds_limit, audio_rate_switch_cover_seconds, audio_rate_switch_min_apply_seconds,
    discontinuity_smoothing_profile, output_staging_frames, rate_switch_cover_output_staging_frames,
    seek_refill_output_staging_frames, seek_settle_output_staging_frames, seek_settle_queue_depth_limit,
    video_drain_batch_limit,
};
use super::constants::{
    DISCONTINUITY_CROSSFADE_FRAMES_BASE, DISCONTINUITY_FADE_IN_FRAMES_BASE,
    OUTPUT_STAGING_FRAMES_AUDIO_ONLY_DEFAULT, OUTPUT_STAGING_FRAMES_AUDIO_ONLY_FAST,
    OUTPUT_STAGING_FRAMES_AUDIO_ONLY_SLOW, OUTPUT_STAGING_FRAMES_VIDEO_DEFAULT, OUTPUT_STAGING_FRAMES_VIDEO_FAST,
    OUTPUT_STAGING_FRAMES_VIDEO_SLOW,
};

#[test]
fn queue_depth_policy_tracks_playback_rate() {
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(1.5), true, false, false),
        3
    );
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(1.25), true, false, false),
        4
    );
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(0.75), true, false, false),
        7
    );
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(1.25), true, true, true),
        10
    );
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(1.0), false, false, false),
        8
    );
    assert_eq!(
        audio_queue_depth_limit(PlaybackRate::new(1.0), false, false, true),
        16
    );
}

#[test]
fn discontinuity_smoothing_scales_with_rate_delta() {
    let neutral = discontinuity_smoothing_profile(PlaybackRate::new(1.0), PlaybackRate::new(1.0));
    let aggressive =
        discontinuity_smoothing_profile(PlaybackRate::new(1.0), PlaybackRate::new(2.0));
    assert_eq!(neutral.fade_in_frames, DISCONTINUITY_FADE_IN_FRAMES_BASE);
    assert!(aggressive.crossfade_frames > DISCONTINUITY_CROSSFADE_FRAMES_BASE);
}

#[test]
fn output_staging_tracks_playback_rate() {
    assert_eq!(
        output_staging_frames(PlaybackRate::new(1.0), true, false),
        OUTPUT_STAGING_FRAMES_VIDEO_DEFAULT
    );
    assert_eq!(
        output_staging_frames(PlaybackRate::new(1.5), true, false),
        OUTPUT_STAGING_FRAMES_VIDEO_FAST
    );
    assert_eq!(
        output_staging_frames(PlaybackRate::new(0.5), true, false),
        OUTPUT_STAGING_FRAMES_VIDEO_SLOW
    );
}

#[test]
fn audio_only_local_output_staging_stays_on_shared_defaults() {
    assert_eq!(
        output_staging_frames(PlaybackRate::new(1.0), false, false),
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_DEFAULT
    );
    assert_eq!(
        output_staging_frames(PlaybackRate::new(1.5), false, false),
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_FAST
    );
    assert_eq!(
        output_staging_frames(PlaybackRate::new(0.5), false, false),
        OUTPUT_STAGING_FRAMES_AUDIO_ONLY_SLOW
    );
}

#[test]
fn rate_switch_cover_output_uses_coarser_blocks() {
    assert_eq!(rate_switch_cover_output_staging_frames(true, true), 1024);
    assert_eq!(rate_switch_cover_output_staging_frames(true, false), 2048);
    assert_eq!(rate_switch_cover_output_staging_frames(false, false), 4096);
}

#[test]
fn seek_refill_output_prefers_low_latency_blocks() {
    assert_eq!(seek_refill_output_staging_frames(true), 768);
    assert_eq!(seek_refill_output_staging_frames(false), 512);
}

#[test]
fn seek_settle_output_uses_coarser_local_audio_blocks() {
    assert_eq!(seek_settle_output_staging_frames(true, false), 1024);
    assert_eq!(seek_settle_output_staging_frames(false, false), 2048);
    assert_eq!(seek_settle_output_staging_frames(false, true), 1024);
}

#[test]
fn seek_settle_queue_limit_stays_tighter_than_default() {
    assert_eq!(seek_settle_queue_depth_limit(8, false, false, false), 6);
    assert_eq!(seek_settle_queue_depth_limit(16, false, false, true), 8);
    assert_eq!(seek_settle_queue_depth_limit(24, true, false, false), 4);
    assert_eq!(seek_settle_queue_depth_limit(10, true, true, true), 10);
}

#[test]
fn audio_prefill_tracks_media_type_and_rate() {
    assert_eq!(
        audio_queue_prefill_target(PlaybackRate::new(1.0), true, false, false),
        5
    );
    assert_eq!(
        audio_queue_prefill_target(PlaybackRate::new(1.0), false, false, false),
        3
    );
    assert_eq!(
        audio_queue_prefill_target(PlaybackRate::new(1.0), false, false, true),
        6
    );
    assert_eq!(
        audio_queue_prefill_target(PlaybackRate::new(0.5), false, false, false),
        5
    );
    assert_eq!(
        audio_queue_prefill_target(PlaybackRate::new(1.0), true, true, true),
        3
    );
}

#[test]
fn rate_switch_cover_seconds_track_source_type() {
    assert!((audio_rate_switch_cover_seconds(false, false, false) - 0.32).abs() < 1e-6);
    assert!((audio_rate_switch_cover_seconds(false, false, true) - 1.20).abs() < 1e-6);
    assert!((audio_rate_switch_cover_seconds(true, false, false) - 0.30).abs() < 1e-6);
    assert!((audio_rate_switch_cover_seconds(true, true, true) - 0.12).abs() < 1e-6);
}

#[test]
fn rate_switch_min_apply_seconds_track_source_type() {
    assert!((audio_rate_switch_min_apply_seconds(false, false, false) - 0.24).abs() < 1e-6);
    assert!((audio_rate_switch_min_apply_seconds(false, false, true) - 1.20).abs() < 1e-6);
    assert!((audio_rate_switch_min_apply_seconds(true, false, false) - 0.16).abs() < 1e-6);
    assert!((audio_rate_switch_min_apply_seconds(true, true, true) - 0.08).abs() < 1e-6);
}

#[test]
fn local_audio_queue_seconds_limit_tracks_rate() {
    assert_eq!(
        audio_queue_seconds_limit(PlaybackRate::new(1.0), false, false, false),
        Some(0.18)
    );
    assert_eq!(
        audio_queue_seconds_limit(PlaybackRate::new(1.5), false, false, false),
        Some(0.16)
    );
    assert_eq!(
        audio_queue_seconds_limit(PlaybackRate::new(0.5), false, false, false),
        Some(0.24)
    );
    assert_eq!(
        audio_queue_seconds_limit(PlaybackRate::new(1.0), false, false, true),
        None
    );
}

#[test]
fn video_queue_refill_floor_prefers_healthier_buffer() {
    let floor =
        audio_queue_refill_floor_seconds(PlaybackRate::new(1.0), true, false, false).unwrap();
    let limit = audio_queue_seconds_limit(PlaybackRate::new(1.0), true, false, false).unwrap();
    assert!(floor >= 0.09);
    assert!(floor < limit);
    assert!(floor > limit * 0.85);
}

#[test]
fn video_drain_batch_limit_protects_audio_queue() {
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), false, None, false, false),
        None
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), true, None, false, false),
        Some(3)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(2), false, false),
        Some(1)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(4), false, false),
        Some(2)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.5), true, Some(5), false, false),
        Some(3)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(8), false, false),
        None
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.25), true, None, true, false),
        Some(1)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.25), true, Some(4), true, false),
        Some(2)
    );
    assert_eq!(
        video_drain_batch_limit(PlaybackRate::new(1.0), true, Some(5), false, true),
        None
    );
}

