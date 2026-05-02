use super::DecodeRuntime;
use crate::app::media::playback::runtime::audio::effective_playback_rate;
use crate::app::media::playback::rate::{
    audio_queue_depth_limit, audio_queue_prefill_target, audio_queue_seconds_limit,
    audio_rate_switch_cover_seconds, audio_rate_switch_min_apply_seconds,
    seek_settle_queue_depth_limit,
};
use crate::app::media::playback::runtime::{
    AUDIO_ALLOWED_LEAD_SECONDS_DEFAULT, AUDIO_ALLOWED_LEAD_SECONDS_DURING_SETTLE,
    MAX_DECODE_LEAD_SECONDS_DEFAULT, MAX_DECODE_LEAD_SECONDS_DURING_SETTLE,
};
use crate::app::media::state::TimingControls;
use std::sync::Arc;
use tauri::AppHandle;

pub(super) fn refresh_audio_rate(
    _app: &AppHandle,
    runtime: &mut DecodeRuntime,
    timing_controls: &Arc<TimingControls>,
) {
    let has_video_stream = runtime.has_video_stream();
    let is_realtime_source = runtime.is_realtime_source;
    let is_network_source = runtime.is_network_source;
    let Some(audio_state) = runtime.audio_pipeline.as_mut() else {
        return;
    };
    let requested_rate = effective_playback_rate(
        timing_controls.playback_rate_value(),
        is_realtime_source,
    );
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
            requested_rate
        }
    };
    let queued_seconds = audio_state.output.queued_duration_seconds();
    let queued_sources = audio_state.output.queue_depth();
    let cover_seconds = audio_rate_switch_cover_seconds(
        has_video_stream,
        is_realtime_source,
        is_network_source,
    );
    let min_apply_seconds = audio_rate_switch_min_apply_seconds(
        has_video_stream,
        is_realtime_source,
        is_network_source,
    );
    let min_apply_queue_depth = audio_queue_prefill_target(
        pending_rate,
        has_video_stream,
        is_realtime_source,
        is_network_source,
    );
    let cover_ready = queued_seconds + 1e-3 >= cover_seconds;
    let min_cover_ready =
        queued_sources >= min_apply_queue_depth && queued_seconds + 1e-3 >= min_apply_seconds;
    if queued_seconds > 0.0 && !cover_ready && !min_cover_ready {
        runtime.loop_state.rate_switch_hold_logged = true;
        return;
    }
    let preserved_queue_depth = queued_sources;
    runtime.loop_state.begin_rate_switch_settle();
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(pending_rate.as_f64());
    }
    audio_state.restart_after_discontinuity(
        runtime.loop_state.last_applied_audio_rate,
        pending_rate,
        preserved_queue_depth > 0,
    );
    runtime.loop_state.reset_audio_sync_state();
    runtime.loop_state.commit_audio_playback_rate(pending_rate);
}

pub(super) fn should_wait_for_rate_switch_drain(
    _app: &AppHandle,
    _runtime: &mut DecodeRuntime,
) -> bool {
    false
}

pub(super) fn should_wait_for_decode_lead(runtime: &DecodeRuntime) -> bool {
    let in_rate_switch_settle = runtime.loop_state.in_rate_switch_settle();
    let max_lead_seconds = if runtime.is_realtime_source {
        if in_rate_switch_settle { 0.12 } else { 0.08 }
    } else if in_rate_switch_settle {
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
    _app: &AppHandle,
    runtime: &mut DecodeRuntime,
) -> bool {
    let has_video_stream = runtime.has_video_stream();
    let in_rate_switch_settle = runtime.loop_state.in_rate_switch_settle();
    let in_seek_refill = runtime.loop_state.in_seek_refill();
    let in_seek_settle = runtime.loop_state.in_seek_settle();
    let Some(audio) = runtime.audio_pipeline.as_mut() else {
        return false;
    };
    if in_seek_refill && !runtime.is_realtime_source {
        if audio.stats.audio_only_backpressure_logged {
            audio.stats.audio_only_backpressure_logged = false;
        }
        return false;
    }
    if runtime.loop_state.pending_audio_rate.is_some() && !in_rate_switch_settle {
        let cover_seconds = audio_rate_switch_cover_seconds(
            has_video_stream,
            runtime.is_realtime_source,
            runtime.is_network_source,
        );
        let queued_seconds = audio.output.queued_duration_seconds();
        if queued_seconds + 1e-3 < cover_seconds {
            if audio.stats.audio_only_backpressure_logged {
                audio.stats.audio_only_backpressure_logged = false;
            }
            return false;
        }
    }
    let queue_depth_limit = audio_queue_depth_limit(
        runtime.loop_state.last_applied_audio_rate,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
    );
    let queue_depth_limit = if in_seek_settle {
        seek_settle_queue_depth_limit(
            queue_depth_limit,
            has_video_stream,
            runtime.is_realtime_source,
            runtime.is_network_source,
        )
    } else {
        queue_depth_limit
    };
    let queue_depth = audio.output.queue_depth();
    let queued_seconds = audio.output.queued_duration_seconds();
    let queue_seconds_limit = audio_queue_seconds_limit(
        runtime.loop_state.last_applied_audio_rate,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
    );
    let resume_queue_depth_limit = audio_queue_resume_depth_limit(
        queue_depth_limit,
        has_video_stream,
        runtime.is_realtime_source,
        runtime.is_network_source,
        in_seek_settle,
    );
    let resume_queue_seconds_limit =
        queue_seconds_limit.map(|seconds| (seconds * 0.6_f64).max(0.05_f64));
    let should_wait = if audio.stats.audio_only_backpressure_logged {
        queue_depth > resume_queue_depth_limit
            && resume_queue_seconds_limit
                .map(|seconds| queued_seconds > seconds)
                .unwrap_or(true)
    } else {
        queue_depth >= queue_depth_limit
            && queue_seconds_limit
                .map(|seconds| queued_seconds >= seconds)
                .unwrap_or(true)
    };
    if should_wait && !audio.stats.audio_only_backpressure_logged {
        audio.stats.audio_only_backpressure_logged = true;
    } else if !should_wait && audio.stats.audio_only_backpressure_logged {
        audio.stats.audio_only_backpressure_logged = false;
    }
    should_wait && (!has_video_stream || runtime.is_realtime_source)
}

fn audio_queue_resume_depth_limit(
    queue_depth_limit: usize,
    has_video_stream: bool,
    is_realtime_source: bool,
    is_network_source: bool,
    in_seek_settle: bool,
) -> usize {
    if queue_depth_limit <= 1 {
        return 0;
    }
    let hysteresis = if in_seek_settle {
        1
    } else if has_video_stream || is_realtime_source {
        2
    } else if is_network_source {
        3
    } else {
        4
    };
    queue_depth_limit.saturating_sub(hysteresis).max(1)
}

pub(super) fn current_audio_allowed_lead_seconds(runtime: &DecodeRuntime) -> f64 {
    if runtime.is_realtime_source {
        if runtime.loop_state.in_rate_switch_settle() {
            0.005
        } else {
            0.0
        }
    } else if runtime.loop_state.in_rate_switch_settle() {
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
    let next_rate = effective_playback_rate(
        timing_controls.playback_rate_value(),
        runtime.is_realtime_source,
    );
    if next_rate.delta(runtime.loop_state.last_applied_audio_rate) <= 1e-3 {
        return;
    }
    if let Some(clock) = runtime.loop_state.audio_clock.as_mut() {
        clock.rebase_rate(next_rate.as_f64());
    }
    audio_state.restart_after_discontinuity(
        runtime.loop_state.last_applied_audio_rate,
        next_rate,
        false,
    );
    runtime.loop_state.commit_audio_playback_rate(next_rate);
}
