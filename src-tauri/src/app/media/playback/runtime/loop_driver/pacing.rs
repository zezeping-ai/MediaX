use super::DecodeRuntime;
use crate::app::media::playback::rate::{
    audio_queue_depth_limit, rate_switch_queue_drain_threshold, PlaybackRate,
};
use crate::app::media::playback::runtime::emit_debug;
use crate::app::media::playback::runtime::{
    AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT, AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE,
    MAX_DECODE_LEAD_SECONDS_DEFAULT, MAX_DECODE_LEAD_SECONDS_DURING_SETTLE,
};
use crate::app::media::state::TimingControls;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn refresh_audio_rate(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
    timing_controls: &Arc<TimingControls>,
) {
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return;
    };
    let requested_rate = timing_controls.playback_rate_value();
    if requested_rate.delta(runtime.loop_state.last_applied_audio_rate) <= 1e-3 {
        runtime.loop_state.clear_pending_audio_rate_switch();
        return;
    }
    let pending_rate = match runtime.loop_state.pending_audio_rate {
        Some(pending_rate) if pending_rate.delta(requested_rate) <= 1e-3 => pending_rate,
        _ => {
            runtime
                .loop_state
                .schedule_audio_rate_switch(requested_rate);
            emit_debug(
                app,
                "audio_rate_switch_schedule",
                format!(
                    "schedule audio rate switch {:.2}x -> {:.2}x queue_sources={}",
                    runtime.loop_state.last_applied_audio_rate.as_f32(),
                    requested_rate.as_f32(),
                    audio_state.output.queue_depth(),
                ),
            );
            requested_rate
        }
    };
    if should_delay_audio_rate_switch(
        audio_state.output.queue_depth(),
        runtime.loop_state.last_applied_audio_rate,
        pending_rate,
    ) {
        return;
    }
    runtime.loop_state.begin_rate_switch_settle();
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(pending_rate.as_f64());
    }
    audio_state.restart_after_discontinuity(
        runtime.loop_state.last_applied_audio_rate,
        pending_rate,
    );
    runtime.loop_state.reset_audio_sync_state();
    runtime.loop_state.commit_audio_playback_rate(pending_rate);
    emit_debug(
        app,
        "audio_rate_switch_apply",
        format!(
            "apply audio rate switch -> {:.2}x after drain queue_sources={}",
            pending_rate.as_f32(),
            audio_state.output.queue_depth(),
        ),
    );
}

pub(super) fn should_wait_for_rate_switch_drain(
    app: &AppHandle,
    runtime: &mut DecodeRuntime,
) -> bool {
    let Some(audio_state) = runtime.audio_pipeline.as_ref() else {
        return false;
    };
    let Some(target_rate) = runtime.loop_state.pending_audio_rate else {
        return false;
    };
    if !should_delay_audio_rate_switch(
        audio_state.output.queue_depth(),
        runtime.loop_state.last_applied_audio_rate,
        target_rate,
    ) {
        runtime.loop_state.rate_switch_hold_logged = false;
        return false;
    }
    if !runtime.loop_state.rate_switch_hold_logged {
        emit_debug(
            app,
            "audio_rate_switch_hold",
            format!(
                "hold packet decode until queue drains for cleaner switch current={:.2}x target={:.2}x queue_sources={}",
                runtime.loop_state.last_applied_audio_rate.as_f32(),
                target_rate.as_f32(),
                audio_state.output.queue_depth(),
            ),
        );
        runtime.loop_state.rate_switch_hold_logged = true;
    }
    true
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
    let queue_depth_limit = audio_queue_depth_limit(runtime.loop_state.last_applied_audio_rate);
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
    let next_rate = timing_controls.playback_rate_value();
    if next_rate.delta(runtime.loop_state.last_applied_audio_rate) <= 1e-3 {
        return;
    }
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(next_rate.as_f64());
    }
    audio_state.restart_after_discontinuity(runtime.loop_state.last_applied_audio_rate, next_rate);
    runtime.loop_state.commit_audio_playback_rate(next_rate);
}

fn should_delay_audio_rate_switch(
    queue_depth: usize,
    current_rate: PlaybackRate,
    target_rate: PlaybackRate,
) -> bool {
    queue_depth > rate_switch_queue_drain_threshold(current_rate, target_rate)
}
