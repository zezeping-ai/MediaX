import type { MediaTelemetryPayload } from "@/modules/media-types";
import { isFiniteNumber } from "./classifiers";

export function resolvePipelineBottleneck(telemetry: MediaTelemetryPayload | null): string {
  if (!telemetry) return "insufficient data";
  const sustain = telemetry.network_sustain_ratio ?? null;
  if (isFiniteNumber(sustain) && sustain < 1) return "network throughput";
  const audioQueue = telemetry.audio_queue_depth_sources ?? null;
  if (typeof audioQueue === "number" && audioQueue < 3) return "audio buffering";
  const gpuUtil = telemetry.gpu_queue_utilization ?? null;
  if (isFiniteNumber(gpuUtil) && gpuUtil >= 0.85) return "render queue backpressure";
  const sourceFps = telemetry.source_fps;
  if (isFiniteNumber(sourceFps) && sourceFps > 0) {
    const frameBudgetMs = 1000 / sourceFps;
    if (
      isFiniteNumber(telemetry.decode_avg_frame_cost_ms) &&
      telemetry.decode_avg_frame_cost_ms >= frameBudgetMs * 0.9
    ) {
      return "decode budget saturation";
    }
    if (
      isFiniteNumber(telemetry.render_estimated_cost_ms) &&
      telemetry.render_estimated_cost_ms >= frameBudgetMs * 0.9
    ) {
      return "render budget saturation";
    }
  }
  return "no dominant bottleneck";
}
