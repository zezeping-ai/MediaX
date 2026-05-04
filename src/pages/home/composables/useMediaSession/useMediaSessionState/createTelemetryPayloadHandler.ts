import type { MediaTelemetryPayload } from "@/modules/media-types";
import type { MediaSessionStateRefs, TelemetryPayloadHandler } from "./types";

const TELEMETRY_STALE_TIMEOUT_MS = 2000;

export function createTelemetryPayloadHandler(
  state: MediaSessionStateRefs,
): TelemetryPayloadHandler {
  return (payload: MediaTelemetryPayload) => {
    const atMs = Date.now();
    state.lastTelemetryAtMs.value = atMs;
    if (state.telemetryStaleTimeoutId.value !== null) {
      window.clearTimeout(state.telemetryStaleTimeoutId.value);
    }
    state.telemetryStaleTimeoutId.value = window.setTimeout(() => {
      if (!state.currentSource.value) {
        return;
      }
      state.networkReadBytesPerSecond.value = 0;
      state.networkSustainRatio.value = null;
      state.progressClockSeconds.value = null;
      state.displayVideoPtsSeconds.value = null;
      state.effectiveDisplayVideoPtsSeconds.value = null;
      state.syncVideoPtsSeconds.value = null;
      state.presentedVideoPtsSeconds.value = null;
      state.submittedVideoPtsSeconds.value = null;
      state.currentAudioClockSeconds.value = null;
      state.syncVideoMinusAudioSeconds.value = null;
      state.telemetryStaleTimeoutId.value = null;
    }, TELEMETRY_STALE_TIMEOUT_MS);

    state.networkReadBytesPerSecond.value =
      typeof payload.network_read_bytes_per_second === "number"
      && Number.isFinite(payload.network_read_bytes_per_second)
        ? Math.max(0, payload.network_read_bytes_per_second)
        : null;
    state.networkSustainRatio.value =
      typeof payload.network_sustain_ratio === "number"
      && Number.isFinite(payload.network_sustain_ratio)
        ? Math.max(0, payload.network_sustain_ratio)
        : null;
    state.progressClockSeconds.value =
      typeof payload.progress_clock_seconds === "number"
      && Number.isFinite(payload.progress_clock_seconds)
        ? Math.max(0, payload.progress_clock_seconds)
        : null;
    state.displayVideoPtsSeconds.value =
      typeof payload.display_video_pts_seconds === "number"
      && Number.isFinite(payload.display_video_pts_seconds)
        ? Math.max(0, payload.display_video_pts_seconds)
        : null;
    state.effectiveDisplayVideoPtsSeconds.value =
      typeof payload.effective_display_video_pts_seconds === "number"
      && Number.isFinite(payload.effective_display_video_pts_seconds)
        ? Math.max(0, payload.effective_display_video_pts_seconds)
        : null;
    state.syncVideoPtsSeconds.value =
      typeof payload.sync_video_pts_seconds === "number"
      && Number.isFinite(payload.sync_video_pts_seconds)
        ? Math.max(0, payload.sync_video_pts_seconds)
        : null;
    state.presentedVideoPtsSeconds.value =
      typeof payload.presented_video_pts_seconds === "number"
      && Number.isFinite(payload.presented_video_pts_seconds)
        ? Math.max(0, payload.presented_video_pts_seconds)
        : null;
    state.submittedVideoPtsSeconds.value =
      typeof payload.submitted_video_pts_seconds === "number"
      && Number.isFinite(payload.submitted_video_pts_seconds)
        ? Math.max(0, payload.submitted_video_pts_seconds)
        : null;
    state.currentAudioClockSeconds.value =
      typeof payload.current_audio_clock_seconds === "number"
      && Number.isFinite(payload.current_audio_clock_seconds)
        ? Math.max(0, payload.current_audio_clock_seconds)
        : null;
    state.syncVideoMinusAudioSeconds.value =
      typeof payload.sync_video_minus_audio_seconds === "number"
      && Number.isFinite(payload.sync_video_minus_audio_seconds)
        ? payload.sync_video_minus_audio_seconds
        : null;
  };
}
