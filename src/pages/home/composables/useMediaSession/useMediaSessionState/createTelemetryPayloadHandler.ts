import type { MediaTelemetryPayload } from "@/modules/media-types";
import type { MediaSessionStateRefs, TelemetryPayloadHandler } from "./types";

export function createTelemetryPayloadHandler(
  state: MediaSessionStateRefs,
): TelemetryPayloadHandler {
  return (payload: MediaTelemetryPayload) => {
    const atMs = Date.now();
    state.lastTelemetryAtMs.value = atMs;

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
