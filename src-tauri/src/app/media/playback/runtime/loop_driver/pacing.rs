use super::DecodeRuntime;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::runtime::{
    AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT, AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE,
    BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT,
    MAX_DECODE_LEAD_SECONDS_DEFAULT, MAX_DECODE_LEAD_SECONDS_DURING_SETTLE,
};
use crate::app::media::state::TimingControls;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn refresh_audio_rate(
    runtime: &mut DecodeRuntime,
    timing_controls: &Arc<TimingControls>,
) {
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return;
    };
    let next_rate = clamp_playback_rate(timing_controls.playback_rate());
    if (next_rate - runtime.loop_state.last_applied_audio_rate).abs() <= 1e-3 {
        return;
    }
    runtime.loop_state.begin_rate_switch_settle();
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(next_rate as f64);
    }
    audio_state.restart_after_discontinuity();
    runtime.loop_state.reset_audio_sync_state();
    runtime.loop_state.commit_audio_playback_rate(next_rate);
}

pub(super) fn should_wait_for_decode_lead(runtime: &DecodeRuntime) -> bool {
    let in_rate_switch_settle = runtime.loop_state.in_rate_switch_settle();
    let max_lead_seconds = if in_rate_switch_settle {
        MAX_DECODE_LEAD_SECONDS_DURING_SETTLE
    } else {
        MAX_DECODE_LEAD_SECONDS_DEFAULT
    };
    let audio_now_seconds = runtime
        .loop_state
        .audio_clock
        .as_ref()
        .map(|clock| clock.now_seconds());
    if let (Some(video_pts), Some(audio_pts)) =
        (runtime.loop_state.last_video_pts_seconds, audio_now_seconds)
    {
        let lead = video_pts - audio_pts;
        return lead.is_finite() && lead > max_lead_seconds;
    }
    false
}

pub(super) fn should_wait_for_audio_queue_drain(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
) -> bool {
    let has_video_stream = runtime.has_video_stream();
    let Some(audio) = runtime.audio_pipeline.as_mut() else {
        return false;
    };
    let queue_depth_limit =
        audio_queue_depth_limit(clamp_playback_rate(runtime.loop_state.last_applied_audio_rate));
    let should_wait = audio.output.queue_depth() >= queue_depth_limit;
    if should_wait && !audio.stats.audio_only_backpressure_logged {
        emit_debug(
            app,
            "audio_queue_backpressure",
            format!(
                "pause packet read to protect shared audio speed pipeline queue_sources={} threshold={} has_video={}",
                audio.output.queue_depth(),
                queue_depth_limit,
                has_video_stream,
            ),
        );
        audio.stats.audio_only_backpressure_logged = true;
    } else if !should_wait && audio.stats.audio_only_backpressure_logged {
        emit_debug(
            app,
            "audio_queue_backpressure",
            format!(
                "resume packet read after shared audio speed pipeline drain queue_sources={} threshold={} has_video={}",
                audio.output.queue_depth(),
                queue_depth_limit,
                has_video_stream,
            ),
        );
        audio.stats.audio_only_backpressure_logged = false;
    }
    !has_video_stream && should_wait
}

pub(super) fn audio_queue_depth_limit(playback_rate: f32) -> usize {
    if playback_rate >= 1.5 {
        14
    } else if playback_rate >= 1.25 {
        18
    } else if playback_rate <= 0.75 {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT.saturating_add(6)
    } else {
        BASE_AUDIO_QUEUE_SOURCE_DEPTH_LIMIT
    }
}

pub(super) fn current_audio_allowed_lead_seconds(runtime: &DecodeRuntime) -> f64 {
    if runtime.loop_state.in_rate_switch_settle() {
        AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE
    } else {
        AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT
    }
}

pub(super) fn refresh_tail_audio_rate(
    runtime: &mut DecodeRuntime,
    timing_controls: &Arc<TimingControls>,
) {
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return;
    };
    let next_rate = clamp_playback_rate(timing_controls.playback_rate());
    if (next_rate - runtime.loop_state.last_applied_audio_rate).abs() <= 1e-3 {
        return;
    }
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(next_rate as f64);
    }
    audio_state.restart_after_discontinuity();
    runtime.loop_state.commit_audio_playback_rate(next_rate);
}

#[cfg(test)]
mod tests {
    use super::audio_queue_depth_limit;

    #[test]
    fn tightens_audio_queue_for_fast_rates() {
        assert_eq!(audio_queue_depth_limit(1.5), 14);
        assert_eq!(audio_queue_depth_limit(1.25), 18);
    }

    #[test]
    fn widens_audio_queue_for_slow_rates() {
        assert_eq!(audio_queue_depth_limit(0.75), 30);
    }
}
