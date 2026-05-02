use super::super::types::AudioPipeline;
use crate::app::media::playback::rate::PlaybackRate;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::{emit_debug, METRICS_EMIT_INTERVAL_MS};
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(super) fn emit_audio_debug_if_needed(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    playback_rate: PlaybackRate,
    channels: usize,
    samples_per_channel: usize,
    queued_samples: usize,
    queued_seconds: f64,
    audio_clock: &Option<AudioClock>,
) {
    let now = Instant::now();
    let should_emit = audio_state
        .stats
        .last_debug_instant
        .map(|last| {
            now.saturating_duration_since(last) >= Duration::from_millis(METRICS_EMIT_INTERVAL_MS)
        })
        .unwrap_or(true);
    if !should_emit {
        return;
    }
    audio_state.stats.last_debug_instant = Some(now);
    emit_debug(
        app,
        "audio_output",
        format!(
            "volume={:.2} muted={} rate={:.2} queue_sources={} queue_seconds={:.3}",
            audio_state.output.controls.volume(),
            audio_state.output.controls.muted(),
            playback_rate.as_f32(),
            audio_state.output.queue_depth(),
            queued_seconds,
        ),
    );
    emit_debug(
        app,
        "audio_stats",
        format!(
            "packets={} frames={} queued_samples={} underruns={} queue_sources={} queue_seconds={:.3} rate={:.2} channels={} samples_per_ch={} output_samples={} out_fmt={} pts={}",
            audio_state.stats.packets,
            audio_state.stats.decoded_frames,
            audio_state.stats.queued_samples,
            audio_state.stats.underrun_count,
            audio_state.output.queue_depth(),
            queued_seconds,
            playback_rate.as_f32(),
            channels,
            samples_per_channel,
            queued_samples,
            audio_state.output_sample_format.debug_label(),
            audio_clock
                .as_ref()
                .map(|clock| clock.now_seconds())
                .map(|value| format!("{value:.3}s"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
    );
}
