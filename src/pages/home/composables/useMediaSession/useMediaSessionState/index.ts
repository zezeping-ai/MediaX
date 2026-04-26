import { ref } from "vue";
import type {
  MediaErrorPayload,
  MediaMetadataPayload,
  MediaSnapshot,
  MediaTelemetryPayload,
} from "@/modules/media-types";
import { toUserMediaErrorMessage } from "../../useMediaErrorMap";
import { createDebugPayloadHandler } from "./createDebugPayloadHandler";
import { createTelemetryPayloadHandler } from "./createTelemetryPayloadHandler";

export function useMediaSessionState() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const debugSnapshot = ref<Record<string, string>>({});
  const debugTimeline = ref<Array<{ stage: string; message: string; at_ms: number }>>([]);
  const debugStageSnapshot = ref<Record<string, { message: string; at_ms: number }>>({});
  const latestTelemetry = ref<MediaTelemetryPayload | null>(null);
  const telemetryHistory = ref<Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>>([]);
  const firstFrameAtMs = ref<number | null>(null);
  const metadataDurationSeconds = ref<number | null>(null);
  const metadataVideoWidth = ref<number | null>(null);
  const metadataVideoHeight = ref<number | null>(null);
  const metadataVideoFps = ref<number | null>(null);
  const playbackErrorMessage = ref("");
  const networkReadBytesPerSecond = ref<number | null>(null);
  const networkSustainRatio = ref<number | null>(null);
  const lastTelemetryAtMs = ref(0);

  function resetTransientMediaState() {
    debugSnapshot.value = {};
    debugTimeline.value = [];
    debugStageSnapshot.value = {};
    firstFrameAtMs.value = null;
    latestTelemetry.value = null;
    telemetryHistory.value = [];
    metadataDurationSeconds.value = null;
    metadataVideoWidth.value = null;
    metadataVideoHeight.value = null;
    metadataVideoFps.value = null;
    networkReadBytesPerSecond.value = null;
    networkSustainRatio.value = null;
    lastTelemetryAtMs.value = 0;
    playbackErrorMessage.value = "";
  }

  function updateSnapshot(next: MediaSnapshot) {
    const previousSource = currentSource.value;
    snapshot.value = next;
    currentSource.value = next.playback.current_path ?? "";
    if (previousSource !== currentSource.value) {
      resetTransientMediaState();
    }
  }

  function applyMetadataPayload(payload: MediaMetadataPayload) {
    metadataDurationSeconds.value = payload.duration_seconds;
    metadataVideoWidth.value = payload.width;
    metadataVideoHeight.value = payload.height;
    metadataVideoFps.value = payload.fps;
  }

  function applyErrorPayload(payload: MediaErrorPayload) {
    playbackErrorMessage.value = toUserMediaErrorMessage(`${payload.code}: ${payload.message}`);
  }

  const applyDebugPayload = createDebugPayloadHandler({
    currentSource,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    latestTelemetry,
    telemetryHistory,
    firstFrameAtMs,
    networkReadBytesPerSecond,
    networkSustainRatio,
    lastTelemetryAtMs,
  });

  const applyTelemetryPayload = createTelemetryPayloadHandler({
    currentSource,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    latestTelemetry,
    telemetryHistory,
    firstFrameAtMs,
    networkReadBytesPerSecond,
    networkSustainRatio,
    lastTelemetryAtMs,
  });

  function markTelemetryStaleIfNeeded() {
    if (!currentSource.value) {
      return;
    }
    if (!lastTelemetryAtMs.value) {
      return;
    }
    if (Date.now() - lastTelemetryAtMs.value >= 2000) {
      networkReadBytesPerSecond.value = 0;
      networkSustainRatio.value = null;
    }
  }

  return {
    applyDebugPayload,
    applyErrorPayload,
    applyMetadataPayload,
    applyTelemetryPayload,
    currentSource,
    debugSnapshot,
    debugStageSnapshot,
    debugTimeline,
    firstFrameAtMs,
    latestTelemetry,
    markTelemetryStaleIfNeeded,
    metadataDurationSeconds,
    metadataVideoFps,
    metadataVideoHeight,
    metadataVideoWidth,
    networkReadBytesPerSecond,
    networkSustainRatio,
    playbackErrorMessage,
    snapshot,
    telemetryHistory,
    updateSnapshot,
  };
}
