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
  };
}
