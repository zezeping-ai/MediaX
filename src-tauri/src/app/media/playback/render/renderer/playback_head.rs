use super::types::{
    VideoPlaybackHeadPosition, VideoPlaybackHeadPrecision, VideoPlaybackHeadSource,
    VideoDisplayReference, VideoPlaybackHeads, VideoPlaybackReferenceSet,
    VideoSyncFallbackKind, VideoSyncReference,
};

const EFFECTIVE_DISPLAY_FALLBACK_PRESENT_LEAD_SECONDS: f64 = 0.004;

pub(super) fn build_playback_heads(
    measured_presented_pts_seconds: Option<f64>,
    queued_head_pts_seconds: Option<f64>,
    present_lead_seconds: Option<f64>,
    now_media_seconds: f64,
) -> VideoPlaybackHeads {
    let measured = measured_presented_pts_seconds
        .filter(|value| value.is_finite() && *value >= 0.0)
        .map(|seconds| VideoPlaybackHeadPosition {
            seconds,
            precision: VideoPlaybackHeadPrecision::Measured,
            source: VideoPlaybackHeadSource::Presented,
        });
    let estimated = estimate_effective_display_head(
        measured.map(|position| position.seconds),
        queued_head_pts_seconds,
        present_lead_seconds.unwrap_or(EFFECTIVE_DISPLAY_FALLBACK_PRESENT_LEAD_SECONDS),
        now_media_seconds,
    )
    .map(|seconds| VideoPlaybackHeadPosition {
        seconds,
        precision: VideoPlaybackHeadPrecision::Estimated,
        source: VideoPlaybackHeadSource::EffectiveDisplay,
    });
    VideoPlaybackHeads {
        measured,
        estimated,
    }
}

pub(crate) fn resolve_playback_references(
    playback_heads: VideoPlaybackHeads,
    estimated_pts_seconds: f64,
) -> VideoPlaybackReferenceSet {
    VideoPlaybackReferenceSet {
        sync: resolve_sync_reference(playback_heads, estimated_pts_seconds),
        display: resolve_display_reference(playback_heads),
    }
}

fn resolve_sync_reference(
    playback_heads: VideoPlaybackHeads,
    estimated_pts_seconds: f64,
) -> VideoSyncReference {
    if let Some(position) = playback_heads.preferred() {
        return VideoSyncReference {
            position,
            fallback: None,
        };
    }
    VideoSyncReference {
        position: VideoPlaybackHeadPosition {
            seconds: estimated_pts_seconds.max(0.0),
            precision: VideoPlaybackHeadPrecision::Estimated,
            source: VideoPlaybackHeadSource::EffectiveDisplay,
        },
        fallback: Some(VideoSyncFallbackKind::EstimatedPts),
    }
}

fn resolve_display_reference(playback_heads: VideoPlaybackHeads) -> VideoDisplayReference {
    VideoDisplayReference {
        position: playback_heads.preferred(),
    }
}

fn estimate_effective_display_head(
    presented_pts_seconds: Option<f64>,
    queued_head_pts_seconds: Option<f64>,
    present_lead_seconds: f64,
    now_media_seconds: f64,
) -> Option<f64> {
    let queued_head_pts_seconds =
        queued_head_pts_seconds.filter(|value| value.is_finite() && *value >= 0.0);
    let next_due_pts = queued_head_pts_seconds
        .filter(|pts| *pts <= now_media_seconds + present_lead_seconds.max(0.0));
    match (presented_pts_seconds, queued_head_pts_seconds, next_due_pts) {
        (_, _, Some(next_due_pts)) => Some(next_due_pts.max(0.0)),
        (Some(presented_pts), Some(queued_head_pts), None) => {
            let queued_gap_seconds = queued_head_pts - presented_pts;
            if (0.060..=0.100).contains(&queued_gap_seconds) {
                Some((presented_pts + queued_gap_seconds * 0.5).max(0.0))
            } else {
                Some(presented_pts.max(0.0))
            }
        }
        (Some(presented_pts), None, None) => Some(presented_pts.max(0.0)),
        (None, Some(queued_head_pts), None) => Some(queued_head_pts.max(0.0)),
        (None, None, None) => None,
    }
}
