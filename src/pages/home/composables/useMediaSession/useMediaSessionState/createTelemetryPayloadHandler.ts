import type { MediaTelemetryPayload } from "@/modules/media-types";
import { MAX_TELEMETRY_HISTORY_SIZE } from "./constants";
import type { MediaSessionStateRefs, TelemetryPayloadHandler } from "./types";

export function createTelemetryPayloadHandler(
  state: MediaSessionStateRefs,
): TelemetryPayloadHandler {
  return (payload: MediaTelemetryPayload) => {
    const atMs = Date.now();
    state.latestTelemetry.value = payload;
    state.telemetryHistory.value = [
      ...state.telemetryHistory.value,
      { at_ms: atMs, telemetry: payload },
    ].slice(-MAX_TELEMETRY_HISTORY_SIZE);
    state.lastTelemetryAtMs.value = atMs;

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
    const videoPacketSoftErrors = payload.video_packet_soft_error_count ?? 0;
    const videoFrameDrops = payload.video_frame_drop_count ?? 0;
    const videoHwDrops = payload.video_hw_transfer_drop_count ?? 0;
    const videoScaleDrops = payload.video_scale_drop_count ?? 0;
    const videoNv12Drops = payload.video_nv12_drop_count ?? 0;

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

    const renderBusyEstimatePercent =
      renderCost > 0 && payload.render_fps > 0
        ? Math.min(100, Math.max(0, (renderCost * payload.render_fps) / 10))
        : 0;
    const decodeBusyEstimatePercent =
      decodeAvg > 0 && payload.render_fps > 0
        ? Math.min(100, Math.max(0, (decodeAvg * payload.render_fps) / 10))
        : 0;

    state.debugSnapshot.value = {
      ...state.debugSnapshot.value,
      telemetry_timing:
        `src=${payload.source_fps.toFixed(2)}fps render=${payload.render_fps.toFixed(2)}fps ` +
        `drift=${(payload.audio_drift_seconds ?? 0).toFixed(3)}s`,
      telemetry_resources:
        `decode_avg=${decodeAvg.toFixed(2)}ms decode_max=${decodeMax.toFixed(2)}ms samples=${decodeSamples} ` +
        `解码忙碌≈${decodeBusyEstimatePercent.toFixed(0)}% cpu=${processCpu.toFixed(1)}% mem=${processMemory.toFixed(1)}MB`,
      telemetry_render:
        `gpu_queue=${gpuQueueDepth}/${gpuQueueCapacity || "?"} (${(gpuQueueUsage * 100).toFixed(0)}%) ` +
        `render_cost≈${renderCost.toFixed(2)}ms 渲染忙碌≈${renderBusyEstimatePercent.toFixed(0)}% present_lag≈${renderLag.toFixed(2)}ms`,
      telemetry_video_resilience:
        `packet_soft_errors=${videoPacketSoftErrors} frame_drops=${videoFrameDrops} ` +
        `hw=${videoHwDrops} scale=${videoScaleDrops} nv12=${videoNv12Drops}`,
    };

    if (payload.video_timestamps) {
      state.debugSnapshot.value.video_timestamps =
        `samples=${payload.video_timestamps.samples} pts_missing=${payload.video_timestamps.pts_missing_ratio_percent.toFixed(2)}% ` +
        `backtrack=${payload.video_timestamps.pts_backtrack_count} jitter_avg=${payload.video_timestamps.jitter_avg_ms.toFixed(3)}ms ` +
        `jitter_max=${payload.video_timestamps.jitter_max_ms.toFixed(3)}ms`;
    }
    if (payload.frame_types) {
      state.debugSnapshot.value.video_frame_types =
        `I=${payload.frame_types.i_ratio_percent.toFixed(1)}% P=${payload.frame_types.p_ratio_percent.toFixed(1)}% ` +
        `B=${payload.frame_types.b_ratio_percent.toFixed(1)}% other=${payload.frame_types.other_ratio_percent.toFixed(1)}% ` +
        `samples=${payload.frame_types.sample_count}`;
    }
    if (payload.decode_quantiles) {
      state.debugSnapshot.value.decode_cost_quantiles =
        `p50=${payload.decode_quantiles.p50_ms.toFixed(3)}ms p95=${payload.decode_quantiles.p95_ms.toFixed(3)}ms ` +
        `p99=${payload.decode_quantiles.p99_ms.toFixed(3)}ms avg=${payload.decode_quantiles.avg_ms.toFixed(3)}ms ` +
        `max=${payload.decode_quantiles.max_ms.toFixed(3)}ms samples=${payload.decode_quantiles.sample_count}`;
    }
  };
}
