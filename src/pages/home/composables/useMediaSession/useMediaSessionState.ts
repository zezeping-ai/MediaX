import { ref } from "vue";
import type {
  MediaDebugPayload,
  MediaErrorPayload,
  MediaMetadataPayload,
  MediaSnapshot,
  MediaTelemetryPayload,
} from "@/modules/media-types";
import { toUserMediaErrorMessage } from "../useMediaErrorMap";

const MAX_DEBUG_TIMELINE_SIZE = 200;
const MAX_TELEMETRY_HISTORY_SIZE = 36;

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
  const lastTelemetryAtMs = ref<number>(0);

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

  function applyDebugPayload(payload: MediaDebugPayload) {
    const stage = payload.stage?.trim() || "debug";
    const msg = payload.message?.trim() || "";
    const atMs = payload.at_ms ?? Date.now();
    debugTimeline.value = [
      ...debugTimeline.value,
      { stage, message: msg || "-", at_ms: atMs },
    ].slice(-MAX_DEBUG_TIMELINE_SIZE);
    debugSnapshot.value = {
      ...debugSnapshot.value,
      [stage]: msg || "-",
    };
    debugStageSnapshot.value = {
      ...debugStageSnapshot.value,
      [stage]: {
        message: msg || "-",
        at_ms: atMs,
      },
    };
    if (
      firstFrameAtMs.value === null
      && (stage === "first_frame" || stage === "video_frame_format" || stage === "video_fps")
    ) {
      firstFrameAtMs.value = atMs;
    }
  }

  function applyTelemetryPayload(payload: MediaTelemetryPayload) {
    const atMs = Date.now();
    latestTelemetry.value = payload;
    telemetryHistory.value = [
      ...telemetryHistory.value,
      { at_ms: atMs, telemetry: payload },
    ].slice(-MAX_TELEMETRY_HISTORY_SIZE);
    lastTelemetryAtMs.value = atMs;

    const decodeAvg = payload.decode_avg_frame_cost_ms ?? 0;
    const decodeMax = payload.decode_max_frame_cost_ms ?? 0;
    const decodeSamples = payload.decode_samples ?? 0;
    const processCpu = payload.process_cpu_percent ?? 0;
    const processMemory = payload.process_memory_mb ?? 0;
    const gpuQueueDepth = payload.gpu_queue_depth ?? payload.queue_depth ?? 0;
    const gpuQueueCapacity = payload.gpu_queue_capacity ?? 0;
    const gpuQueueUsage = payload.gpu_queue_utilization ?? 0;
    const renderCost = payload.render_estimated_cost_ms ?? 0;
    const renderLag = payload.render_present_lag_ms ?? 0;

    networkReadBytesPerSecond.value =
      typeof payload.network_read_bytes_per_second === "number"
      && Number.isFinite(payload.network_read_bytes_per_second)
        ? Math.max(0, payload.network_read_bytes_per_second)
        : null;
    networkSustainRatio.value =
      typeof payload.network_sustain_ratio === "number" && Number.isFinite(payload.network_sustain_ratio)
        ? Math.max(0, payload.network_sustain_ratio)
        : null;

    const renderBusyEstimatePercent =
      renderCost > 0 && payload.render_fps > 0
        ? Math.min(100, Math.max(0, (renderCost * payload.render_fps) / 10))
        : 0;
    const decodeBusyEstimatePercent =
      decodeAvg > 0 && payload.render_fps > 0
        ? Math.min(100, Math.max(0, (decodeAvg * payload.render_fps) / 10))
        : 0;

    debugSnapshot.value = {
      ...debugSnapshot.value,
      telemetry_timing:
        `src=${payload.source_fps.toFixed(2)}fps render=${payload.render_fps.toFixed(2)}fps ` +
        `drift=${(payload.audio_drift_seconds ?? 0).toFixed(3)}s`,
      telemetry_resources:
        `decode_avg=${decodeAvg.toFixed(2)}ms decode_max=${decodeMax.toFixed(2)}ms samples=${decodeSamples} ` +
        `解码忙碌≈${decodeBusyEstimatePercent.toFixed(0)}% cpu=${processCpu.toFixed(1)}% mem=${processMemory.toFixed(1)}MB`,
      telemetry_render:
        `gpu_queue=${gpuQueueDepth}/${gpuQueueCapacity || "?"} (${(gpuQueueUsage * 100).toFixed(0)}%) ` +
        `render_cost≈${renderCost.toFixed(2)}ms 渲染忙碌≈${renderBusyEstimatePercent.toFixed(0)}% present_lag≈${renderLag.toFixed(2)}ms`,
    };

    if (payload.video_timestamps) {
      debugSnapshot.value.video_timestamps =
        `samples=${payload.video_timestamps.samples} pts_missing=${payload.video_timestamps.pts_missing_ratio_percent.toFixed(2)}% ` +
        `backtrack=${payload.video_timestamps.pts_backtrack_count} jitter_avg=${payload.video_timestamps.jitter_avg_ms.toFixed(3)}ms ` +
        `jitter_max=${payload.video_timestamps.jitter_max_ms.toFixed(3)}ms`;
    }
    if (payload.frame_types) {
      debugSnapshot.value.video_frame_types =
        `I=${payload.frame_types.i_ratio_percent.toFixed(1)}% P=${payload.frame_types.p_ratio_percent.toFixed(1)}% ` +
        `B=${payload.frame_types.b_ratio_percent.toFixed(1)}% other=${payload.frame_types.other_ratio_percent.toFixed(1)}% ` +
        `samples=${payload.frame_types.sample_count}`;
    }
    if (payload.decode_quantiles) {
      debugSnapshot.value.decode_cost_quantiles =
        `p50=${payload.decode_quantiles.p50_ms.toFixed(3)}ms p95=${payload.decode_quantiles.p95_ms.toFixed(3)}ms ` +
        `p99=${payload.decode_quantiles.p99_ms.toFixed(3)}ms avg=${payload.decode_quantiles.avg_ms.toFixed(3)}ms ` +
        `max=${payload.decode_quantiles.max_ms.toFixed(3)}ms samples=${payload.decode_quantiles.sample_count}`;
    }
  }

  function markTelemetryStaleIfNeeded() {
    if (!currentSource.value) return;
    const last = lastTelemetryAtMs.value;
    if (!last) return;
    if (Date.now() - last >= 2000) {
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
