use super::DecodeRuntime;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::playback::runtime::audio_pipeline;
use crate::app::media::playback::runtime::{
    AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT, AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE,
    MAX_DECODE_LEAD_SECONDS_DEFAULT, MAX_DECODE_LEAD_SECONDS_DURING_SETTLE,
};
use crate::app::media::state::TimingControls;
use std::sync::Arc;

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
    audio_state.output.set_speed(next_rate);
    let queued_sources = audio_state.output.queue_depth();
    if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD && next_rate < 1.0 {
        audio_state.output.clear_queue();
        runtime.loop_state.audio_clock = None;
        runtime.loop_state.audio_queue_depth_sources = None;
    }
    runtime.loop_state.last_applied_audio_rate = next_rate;
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
    audio_state.output.set_speed(next_rate);
    let queued_sources = audio_state.output.queue_depth();
    if queued_sources >= audio_pipeline::DEEP_AUDIO_QUEUE_SOURCE_THRESHOLD && next_rate < 1.0 {
        audio_state.output.clear_queue();
    }
    runtime.loop_state.last_applied_audio_rate = next_rate;
}
