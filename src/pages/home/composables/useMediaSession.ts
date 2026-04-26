import { ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_PLAYBACK_DEBUG_EVENT,
  MEDIA_PLAYBACK_ERROR_EVENT,
  MEDIA_PLAYBACK_METADATA_EVENT,
  MEDIA_PLAYBACK_STATE_EVENT,
  MEDIA_PLAYBACK_TELEMETRY_EVENT,
  MEDIA_MENU_EVENT,
  type MediaEventEnvelope,
  type MediaDebugPayload,
  type MediaErrorPayload,
  type MediaMetadataPayload,
  type MediaSnapshot,
  type MediaTelemetryPayload,
} from "@/modules/media-types";
import { toUserMediaErrorMessage } from "./useMediaErrorMap";

export function useMediaSession() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const debugSnapshot = ref<Record<string, string>>({});
  const debugTimeline = ref<Array<{ stage: string; message: string; at_ms: number }>>([]);
  const firstFrameAtMs = ref<number | null>(null);
  const metadataDurationSeconds = ref<number | null>(null);
  const metadataVideoWidth = ref<number | null>(null);
  const metadataVideoHeight = ref<number | null>(null);
  const metadataVideoFps = ref<number | null>(null);
  const playbackErrorMessage = ref("");
  const networkReadBytesPerSecond = ref<number | null>(null);
  const networkSustainRatio = ref<number | null>(null);
  const lastTelemetryAtMs = ref<number>(0);
  let unlistenPlaybackStateEvent: UnlistenFn | null = null;
  let unlistenMenuEvent: UnlistenFn | null = null;
  let unlistenPlaybackMetadataEvent: UnlistenFn | null = null;
  let unlistenPlaybackErrorEvent: UnlistenFn | null = null;
  let unlistenPlaybackDebugEvent: UnlistenFn | null = null;
  let unlistenPlaybackTelemetryEvent: UnlistenFn | null = null;
  let snapshotPollingTimer: number | null = null;
  let telemetryStaleTimer: number | null = null;

  function resetTransientMediaState() {
    debugSnapshot.value = {};
    debugTimeline.value = [];
    firstFrameAtMs.value = null;
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

  function resolvePayload<T>(payload: T | MediaEventEnvelope<T>): T {
    if (payload && typeof payload === "object" && "payload" in payload) {
      return (payload as MediaEventEnvelope<T>).payload;
    }
    return payload as T;
  }

  async function mount(
    onMenuAction: (action: string) => void,
    getSnapshot: () => Promise<MediaSnapshot>,
  ) {
    updateSnapshot(await getSnapshot());
    unlistenPlaybackStateEvent = await listen<MediaEventEnvelope<MediaSnapshot> | MediaSnapshot>(
      MEDIA_PLAYBACK_STATE_EVENT,
      (event) => {
        updateSnapshot(resolvePayload(event.payload));
      },
    );
    unlistenMenuEvent = await listen<string>(MEDIA_MENU_EVENT, (event) => {
      onMenuAction(event.payload);
    });
    unlistenPlaybackMetadataEvent = await listen<
      MediaEventEnvelope<MediaMetadataPayload> | MediaMetadataPayload
    >(
      MEDIA_PLAYBACK_METADATA_EVENT,
      (event) => {
        const payload = resolvePayload(event.payload);
        metadataDurationSeconds.value = payload.duration_seconds;
        metadataVideoWidth.value = payload.width;
        metadataVideoHeight.value = payload.height;
        metadataVideoFps.value = payload.fps;
      },
    );
    unlistenPlaybackErrorEvent = await listen<MediaEventEnvelope<MediaErrorPayload> | MediaErrorPayload>(
      MEDIA_PLAYBACK_ERROR_EVENT,
      (event) => {
        const payload = resolvePayload(event.payload);
        playbackErrorMessage.value = toUserMediaErrorMessage(`${payload.code}: ${payload.message}`);
      },
    );
    const upsertDebug = (payload: MediaDebugPayload) => {
      const stage = payload.stage?.trim() || "debug";
      const msg = payload.message?.trim() || "";
      const atMs = payload.at_ms ?? Date.now();
      debugTimeline.value = [
        ...debugTimeline.value,
        { stage, message: msg || "-", at_ms: atMs },
      ].slice(-200);
      debugSnapshot.value = {
        ...debugSnapshot.value,
        [stage]: msg || "-",
      };
      if (
        firstFrameAtMs.value === null
        && (stage === "video_frame_format" || stage === "video_pipeline" || stage === "video_fps")
      ) {
        firstFrameAtMs.value = atMs;
      }
    };
    unlistenPlaybackDebugEvent = await listen<MediaEventEnvelope<MediaDebugPayload> | MediaDebugPayload>(
      MEDIA_PLAYBACK_DEBUG_EVENT,
      (event) => {
        upsertDebug(resolvePayload(event.payload));
      },
    );
    unlistenPlaybackTelemetryEvent = await listen<
      MediaEventEnvelope<MediaTelemetryPayload> | MediaTelemetryPayload
    >(
      MEDIA_PLAYBACK_TELEMETRY_EVENT,
      (event) => {
        const p = resolvePayload(event.payload);
        lastTelemetryAtMs.value = Date.now();
        const decodeAvg = p.decode_avg_frame_cost_ms ?? 0;
        const decodeMax = p.decode_max_frame_cost_ms ?? 0;
        const decodeSamples = p.decode_samples ?? 0;
        const processCpu = p.process_cpu_percent ?? 0;
        const processMemory = p.process_memory_mb ?? 0;
        const gpuQueueDepth = p.gpu_queue_depth ?? p.queue_depth ?? 0;
        const gpuQueueCapacity = p.gpu_queue_capacity ?? 0;
        const gpuQueueUsage = p.gpu_queue_utilization ?? 0;
        const renderCost = p.render_estimated_cost_ms ?? 0;
        const renderLag = p.render_present_lag_ms ?? 0;
        networkReadBytesPerSecond.value =
          typeof p.network_read_bytes_per_second === "number" && Number.isFinite(p.network_read_bytes_per_second)
            ? Math.max(0, p.network_read_bytes_per_second)
            : null;
        networkSustainRatio.value =
          typeof p.network_sustain_ratio === "number" && Number.isFinite(p.network_sustain_ratio)
            ? Math.max(0, p.network_sustain_ratio)
            : null;
        const renderBusyEstimatePercent =
          renderCost > 0 && p.render_fps > 0
            ? Math.min(100, Math.max(0, (renderCost * p.render_fps) / 10))
            : 0;
        const decodeBusyEstimatePercent =
          decodeAvg > 0 && p.render_fps > 0
            ? Math.min(100, Math.max(0, (decodeAvg * p.render_fps) / 10))
            : 0;
        debugSnapshot.value = {
          ...debugSnapshot.value,
          telemetry_timing:
            `src=${p.source_fps.toFixed(2)}fps render=${p.render_fps.toFixed(2)}fps ` +
            `drift=${(p.audio_drift_seconds ?? 0).toFixed(3)}s`,
          telemetry_resources:
            `decode_avg=${decodeAvg.toFixed(2)}ms decode_max=${decodeMax.toFixed(2)}ms samples=${decodeSamples} ` +
            `解码忙碌≈${decodeBusyEstimatePercent.toFixed(0)}% cpu=${processCpu.toFixed(1)}% mem=${processMemory.toFixed(1)}MB`,
          telemetry_render:
            `gpu_queue=${gpuQueueDepth}/${gpuQueueCapacity || "?"} (${(gpuQueueUsage * 100).toFixed(0)}%) ` +
            `render_cost≈${renderCost.toFixed(2)}ms 渲染忙碌≈${renderBusyEstimatePercent.toFixed(0)}% present_lag≈${renderLag.toFixed(2)}ms`,
        };
      },
    );
    snapshotPollingTimer = window.setInterval(() => {
      void getSnapshot().then(updateSnapshot);
    }, 1000);

    // If telemetry stops arriving (pause/stop or network stall), avoid showing stale last value forever.
    telemetryStaleTimer = window.setInterval(() => {
      if (!currentSource.value) return;
      const last = lastTelemetryAtMs.value;
      if (!last) return;
      if (Date.now() - last >= 2000) {
        networkReadBytesPerSecond.value = 0;
      }
      if (Date.now() - last >= 2000) {
        networkSustainRatio.value = null;
      }
    }, 500);
  }

  function unmount() {
    unlistenPlaybackStateEvent?.();
    unlistenPlaybackStateEvent = null;
    unlistenMenuEvent?.();
    unlistenMenuEvent = null;
    unlistenPlaybackMetadataEvent?.();
    unlistenPlaybackMetadataEvent = null;
    unlistenPlaybackErrorEvent?.();
    unlistenPlaybackErrorEvent = null;
    unlistenPlaybackDebugEvent?.();
    unlistenPlaybackDebugEvent = null;
    unlistenPlaybackTelemetryEvent?.();
    unlistenPlaybackTelemetryEvent = null;
    if (snapshotPollingTimer !== null) {
      window.clearInterval(snapshotPollingTimer);
      snapshotPollingTimer = null;
    }
    if (telemetryStaleTimer !== null) {
      window.clearInterval(telemetryStaleTimer);
      telemetryStaleTimer = null;
    }
  }

  return {
    snapshot,
    currentSource,
    debugSnapshot,
    debugTimeline,
    firstFrameAtMs,
    networkReadBytesPerSecond,
    networkSustainRatio,
    metadataDurationSeconds,
    metadataVideoWidth,
    metadataVideoHeight,
    metadataVideoFps,
    playbackErrorMessage,
    mount,
    unmount,
    updateSnapshot,
  };
}
