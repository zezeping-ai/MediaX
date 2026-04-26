use super::types::AudioPipeline;
use crate::app::media::playback::render::pts::timestamp_to_seconds;
use crate::app::media::playback::runtime::audio::clamp_playback_rate;
use crate::app::media::playback::runtime::clock::AudioClock;
use crate::app::media::playback::runtime::{emit_debug, METRICS_EMIT_INTERVAL_MS};
use crate::app::media::state::TimingControls;
use ffmpeg_next::frame;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::AppHandle;

pub(crate) fn drain_audio_frames(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    stop_flag: &Arc<AtomicBool>,
    timing_controls: &Arc<TimingControls>,
    audio_clock: &mut Option<AudioClock>,
    audio_queue_depth_sources: &mut Option<usize>,
    active_seek_target_seconds: &mut Option<f64>,
) -> Result<(), String> {
    audio_state.stats.packets = audio_state.stats.packets.saturating_add(1);
    let mut decoded = frame::Audio::empty();
    while audio_state.decoder.receive_frame(&mut decoded).is_ok() {
        if stop_flag.load(Ordering::Relaxed) {
            return Ok(());
        }
        audio_state.stats.decoded_frames = audio_state.stats.decoded_frames.saturating_add(1);
        let mut converted = frame::Audio::empty();
        audio_state
            .resampler
            .run(&decoded, &mut converted)
            .map_err(|err| format!("audio resample failed: {err}"))?;

        let channels = converted.channels().max(1) as usize;
        let samples_per_channel = converted.samples();
        let total_samples = samples_per_channel.saturating_mul(channels);
        if total_samples == 0 {
            continue;
        }
        let bytes_per_sample = std::mem::size_of::<i16>();
        let expected_bytes = total_samples.saturating_mul(bytes_per_sample);
        let data = converted.data(0);
        if data.is_empty() {
            continue;
        }
        let clamped_bytes = expected_bytes.min(data.len());
        if clamped_bytes < bytes_per_sample {
            continue;
        }

        audio_state
            .output
            .player
            .set_speed(clamp_playback_rate(timing_controls.playback_rate()));
        if audio_state.output.player.len() == 0 {
            audio_state.stats.underrun_count = audio_state.stats.underrun_count.saturating_add(1);
        }
        if audio_state.output.player.is_paused() {
            audio_state.output.player.play();
            emit_debug(app, "audio_resume", "audio player resumed from paused state");
        }
        sync_audio_clock(
            &decoded,
            audio_state.time_base,
            timing_controls,
            audio_clock,
            active_seek_target_seconds,
        );

        let pcm: Vec<i16> = data[..clamped_bytes]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        audio_state.stats.queued_samples = audio_state
            .stats
            .queued_samples
            .saturating_add(pcm.len() as u64);
        audio_state
            .output
            .append_pcm_i16(converted.rate(), converted.channels(), &pcm);
        *audio_queue_depth_sources = Some(audio_state.output.player.len());

        emit_audio_debug_if_needed(
            app,
            audio_state,
            timing_controls,
            channels,
            samples_per_channel,
            clamped_bytes,
            audio_clock,
        );
    }
    Ok(())
}

fn sync_audio_clock(
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
            *audio_clock = Some(AudioClock {
                anchor_instant: Instant::now(),
                anchor_media_seconds: seconds,
                anchor_rate: timing_controls.playback_rate().max(0.25) as f64,
            });
        }
    }
}

fn emit_audio_debug_if_needed(
    app: &AppHandle,
    audio_state: &mut AudioPipeline,
    timing_controls: &Arc<TimingControls>,
    channels: usize,
    samples_per_channel: usize,
    clamped_bytes: usize,
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
            "volume={:.2} muted={} rate={:.2} queue_sources={}",
            audio_state.output.controls.volume(),
            audio_state.output.controls.muted(),
            timing_controls.playback_rate(),
            audio_state.output.player.len()
        ),
    );
    emit_debug(
        app,
        "audio_stats",
        format!(
            "packets={} frames={} queued_samples={} underruns={} queue_sources={} rate={:.2} channels={} samples_per_ch={} bytes={} pts={}",
            audio_state.stats.packets,
            audio_state.stats.decoded_frames,
            audio_state.stats.queued_samples,
            audio_state.stats.underrun_count,
            audio_state.output.player.len(),
            timing_controls.playback_rate(),
            channels,
            samples_per_channel,
            clamped_bytes,
            audio_clock
                .as_ref()
                .map(|clock| clock.now_seconds())
                .map(|value| format!("{value:.3}s"))
                .unwrap_or_else(|| "n/a".to_string()),
        ),
    );
}
