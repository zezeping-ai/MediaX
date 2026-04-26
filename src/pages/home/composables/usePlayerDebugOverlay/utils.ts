import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { DEBUG_LABELS } from "./constants";
import type { DebugRow, HardwareDecisionEvent } from "./types";

export function isFiniteNumber(value: number | null | undefined): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

export function pushSnapshotRow(
  rows: DebugRow[],
  snapshot: Record<string, string>,
  key: string,
  label = key,
) {
  const value = snapshot[key];
  if (!value) {
    return;
  }
  rows.push({ key, label, value });
}

export function formatBytesPerSecond(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B/s";
  }
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let size = value;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  return `${size.toFixed(size >= 100 ? 0 : size >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

export function classifyNetworkPressure(sustainRatio: number): string {
  if (!Number.isFinite(sustainRatio)) return "unknown";
  if (sustainRatio >= 1.15) return "healthy";
  if (sustainRatio >= 1.0) return "tight";
  return "underflow risk";
}

export function classifyAudioQueueState(queueDepth: number): string {
  if (!Number.isFinite(queueDepth)) return "unknown";
  if (queueDepth < 3) return "low buffer";
  if (queueDepth < 8) return "stable";
  return "deep buffer";
}

export function classifyGpuQueueState(queueDepth: number, queueCapacity: number | null): string {
  if (!Number.isFinite(queueDepth)) return "unknown";
  if (!queueCapacity || queueCapacity <= 0) {
    return queueDepth > 0 ? "active" : "idle";
  }
  const utilization = queueDepth / queueCapacity;
  if (utilization >= 0.85) return "backpressure";
  if (utilization >= 0.5) return "busy";
  return "headroom";
}

export function classifyBudgetState(costMs: number, budgetMs: number): string {
  if (!Number.isFinite(costMs) || !Number.isFinite(budgetMs) || budgetMs <= 0) {
    return "unknown";
  }
  const ratio = costMs / budgetMs;
  if (ratio >= 1) return "over budget";
  if (ratio >= 0.85) return "tight";
  if (ratio >= 0.5) return "healthy";
  return "ample headroom";
}

export function resolveHardwareCapabilityVerdict(
  playback: PlaybackState | null,
  snapshot: Record<string, string>,
): string {
  if (!playback) return "unknown";
  if (playback.hw_decode_active) {
    return `active via ${playback.hw_decode_backend || "hardware backend"}`;
  }
  if (playback.hw_decode_mode === "off") return "disabled by preference";
  if (snapshot.hw_decode_fallback) return "fallback to software after hardware attempt";
  if (playback.hw_decode_error) return `software only: ${playback.hw_decode_error}`;
  if (snapshot.hw_decode_decision) return snapshot.hw_decode_decision;
  return playback.hw_decode_mode === "on"
    ? "hardware requested; waiting for result"
    : "auto mode; waiting for decision";
}

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
    if (isFiniteNumber(telemetry.decode_avg_frame_cost_ms) && telemetry.decode_avg_frame_cost_ms >= frameBudgetMs * 0.9) {
      return "decode budget saturation";
    }
    if (isFiniteNumber(telemetry.render_estimated_cost_ms) && telemetry.render_estimated_cost_ms >= frameBudgetMs * 0.9) {
      return "render budget saturation";
    }
  }
  return "no dominant bottleneck";
}

export function formatHardwareDecisionLabel(stage: string): string {
  switch (stage) {
    case "open":
      return "打开源";
    case "decoder_ready":
      return "解码器";
    case "hw_decode_decision":
      return "硬解决策";
    case "hw_decode_fallback":
      return "硬解回退";
    case "decode_error":
      return "解码错误";
    default:
      return stage;
  }
}

export function resolveHardwareDecisionTone(
  stage: string,
  message: string,
): HardwareDecisionEvent["tone"] {
  if (stage === "decode_error") return "error";
  if (stage === "hw_decode_fallback") return "warn";
  if (stage === "hw_decode_decision" && message.toLowerCase().includes("hardware decode selected")) {
    return "good";
  }
  return "neutral";
}

export function formatDebugLabel(key: string): string {
  return DEBUG_LABELS[key] || key;
}

export function formatHwModeLabel(mode: string): string {
  switch (mode) {
    case "auto":
      return "自动";
    case "on":
      return "硬解优先";
    case "off":
      return "仅软解";
    default:
      return mode;
  }
}

export function detectDebugGroup(key: string): string {
  if (key === "open" || key === "video_demux" || key === "video_gop") return "input";
  if (
    key === "metadata_ready" ||
    key === "video_format" ||
    key === "video_stream" ||
    key === "audio" ||
    key === "audio_format"
  ) {
    return "stream";
  }
  if (
    key === "decoder_ready" ||
    key === "hw_decode_decision" ||
    key === "audio_pipeline_ready" ||
    key.startsWith("decode")
  ) {
    return "decode";
  }
  if (
    key === "running" ||
    key === "first_frame" ||
    key === "video_pipeline" ||
    key === "video_integrity" ||
    key === "video_fps" ||
    key === "video_gap" ||
    key === "video_frame_types" ||
    key === "decode_cost_quantiles" ||
    key === "color_profile" ||
    key === "color_profill" ||
    key === "color_profile_drift" ||
    key === "hw_frame_transfer" ||
    key === "nv12_extract"
  ) {
    return "video";
  }
  if (
    key === "audio_stats" ||
    key === "audio_output" ||
    key === "audio_resume" ||
    key === "audio_silent"
  ) {
    return "audio";
  }
  if (
    key === "telemetry_timing" ||
    key === "telemetry_render" ||
    key === "telemetry_resources" ||
    key === "seek" ||
    key === "av_sync" ||
    key === "video_timestamps"
  ) {
    return "timing";
  }
  if (key.endsWith("error") || key === "hw_decode_fallback") return "error";
  return "other";
}

export function formatGroupTitle(groupId: string): string {
  switch (groupId) {
    case "input":
      return "输入/流";
    case "stream":
      return "流信息";
    case "decode":
      return "解码";
    case "video":
      return "视频";
    case "audio":
      return "音频";
    case "timing":
      return "时序/性能";
    case "error":
      return "异常";
    default:
      return "其他";
  }
}
