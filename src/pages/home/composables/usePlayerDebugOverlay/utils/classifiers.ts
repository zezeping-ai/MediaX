export function isFiniteNumber(value: number | null | undefined): value is number {
  return typeof value === "number" && Number.isFinite(value);
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
