import type { MediaTelemetryPayload } from "@/modules/media-types";
import type { HealthLane } from "./pipelineHealthPanel.types";

export function buildPipelineHealthLanes(
  telemetry: MediaTelemetryPayload | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane[] {
  if (!telemetry) {
    return [];
  }
  const sourceFps = telemetry.source_fps > 0 ? telemetry.source_fps : 0;
  const frameBudgetMs = sourceFps > 0 ? 1000 / sourceFps : null;
  const networkRatio = telemetry.network_sustain_ratio ?? null;
  const decodeAvg = telemetry.decode_avg_frame_cost_ms ?? null;
  const renderCost = telemetry.render_estimated_cost_ms ?? null;
  const audioQueue = telemetry.audio_queue_depth_sources ?? null;

  return [
    buildNetworkLane(networkRatio, history),
    buildBudgetLane("decode", "解码", decodeAvg, frameBudgetMs, history),
    buildAudioLane(audioQueue, history),
    buildRenderLane(
      telemetry.gpu_queue_utilization ?? null,
      renderCost,
      frameBudgetMs,
      history,
    ),
  ];
}

function buildNetworkLane(
  ratio: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  const normalized = ratio && Number.isFinite(ratio) ? Math.max(0, Math.min(1.2, ratio)) : 0;
  const percent = Math.min(100, normalized * 100);
  const samples = history.map((item) => item.telemetry.network_sustain_ratio ?? 0);
  const trend = resolveTrend(samples, 1.2);
  if (ratio === null || !Number.isFinite(ratio)) {
    return lane("network", "网络", 0, "unknown", "等待带宽采样", "from-slate-500/60 to-slate-400/25", buildSparkline([], 1.2), "pending", "—", [83.33, 95.83]);
  }
  if (ratio >= 1.15) {
    return lane("network", "网络", percent, "healthy", `sustain ${ratio.toFixed(2)}x`, "from-emerald-400/85 to-emerald-300/35", buildSparkline(samples, 1.2), trend.label, trend.deltaLabel, [83.33, 95.83]);
  }
  if (ratio >= 1.0) {
    return lane("network", "网络", percent, "tight", `sustain ${ratio.toFixed(2)}x`, "from-amber-400/85 to-amber-300/35", buildSparkline(samples, 1.2), trend.label, trend.deltaLabel, [83.33, 95.83]);
  }
  return lane("network", "网络", percent, "underflow risk", `sustain ${ratio.toFixed(2)}x`, "from-rose-400/85 to-rose-300/35", buildSparkline(samples, 1.2), trend.label, trend.deltaLabel, [83.33, 95.83]);
}

function buildBudgetLane(
  id: string,
  label: string,
  costMs: number | null,
  budgetMs: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  if (costMs === null || budgetMs === null || !Number.isFinite(costMs) || !Number.isFinite(budgetMs) || budgetMs <= 0) {
    return lane(id, label, 0, "unknown", "等待预算采样", "from-slate-500/60 to-slate-400/25", buildSparkline([], 1), "pending", "—", [50, 85]);
  }
  const ratio = costMs / budgetMs;
  const percent = Math.min(100, Math.max(0, ratio * 100));
  const samples = history.map((item) => {
    const fps = item.telemetry.source_fps > 0 ? item.telemetry.source_fps : 0;
    const budget = fps > 0 ? 1000 / fps : null;
    const cost = id === "decode" ? item.telemetry.decode_avg_frame_cost_ms : item.telemetry.render_estimated_cost_ms;
    return budget && cost !== null && Number.isFinite(cost) ? cost / budget : 0;
  });
  const trend = resolveTrend(samples, 1);
  if (ratio >= 1) {
    return lane(id, label, percent, "over budget", `${costMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms`, "from-rose-400/85 to-rose-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  if (ratio >= 0.85) {
    return lane(id, label, percent, "tight", `${costMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms`, "from-amber-400/85 to-amber-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  if (ratio >= 0.5) {
    return lane(id, label, percent, "healthy", `${costMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms`, "from-emerald-400/85 to-emerald-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  return lane(id, label, percent, "ample headroom", `${costMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms`, "from-cyan-400/85 to-sky-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
}

function buildAudioLane(
  queueDepth: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  if (queueDepth === null || !Number.isFinite(queueDepth)) {
    return lane("audio", "音频", 0, "unknown", "等待音频队列", "from-slate-500/60 to-slate-400/25", buildSparkline([], 12), "pending", "—", [25, 66.67]);
  }
  const percent = Math.min(100, Math.max(8, (queueDepth / 12) * 100));
  const samples = history.map((item) => item.telemetry.audio_queue_depth_sources ?? 0);
  const trend = resolveTrend(samples, 12);
  if (queueDepth < 3) {
    return lane("audio", "音频", percent, "low buffer", `${queueDepth} buffers`, "from-rose-400/85 to-rose-300/35", buildSparkline(samples, 12), trend.label, trend.deltaLabel, [25, 66.67]);
  }
  if (queueDepth < 8) {
    return lane("audio", "音频", percent, "stable", `${queueDepth} buffers`, "from-emerald-400/85 to-emerald-300/35", buildSparkline(samples, 12), trend.label, trend.deltaLabel, [25, 66.67]);
  }
  return lane("audio", "音频", percent, "deep buffer", `${queueDepth} buffers`, "from-cyan-400/85 to-sky-300/35", buildSparkline(samples, 12), trend.label, trend.deltaLabel, [25, 66.67]);
}

function buildRenderLane(
  queueUtilization: number | null,
  renderCostMs: number | null,
  budgetMs: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  const queueRatio = queueUtilization && Number.isFinite(queueUtilization) ? queueUtilization : 0;
  const costRatio =
    renderCostMs !== null && budgetMs !== null && Number.isFinite(renderCostMs) && Number.isFinite(budgetMs) && budgetMs > 0
      ? renderCostMs / budgetMs
      : 0;
  const ratio = Math.max(queueRatio, costRatio);
  const percent = Math.min(100, Math.max(0, ratio * 100));
  const samples = history.map((item) => {
    const fps = item.telemetry.source_fps > 0 ? item.telemetry.source_fps : 0;
    const budget = fps > 0 ? 1000 / fps : null;
    const queue = item.telemetry.gpu_queue_utilization ?? 0;
    const cost = item.telemetry.render_estimated_cost_ms;
    const budgetRatio = budget && cost !== null && Number.isFinite(cost) ? cost / budget : 0;
    return Math.max(queue, budgetRatio);
  });
  const trend = resolveTrend(samples, 1);
  if (ratio >= 1 || queueRatio >= 0.85) {
    return lane("render", "渲染", percent, "backpressure", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms` : "queue saturated", "from-rose-400/85 to-rose-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  if (ratio >= 0.85) {
    return lane("render", "渲染", percent, "tight", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms` : "near budget", "from-amber-400/85 to-amber-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  if (ratio >= 0.5) {
    return lane("render", "渲染", percent, "healthy", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms` : "steady", "from-emerald-400/85 to-emerald-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
  }
  return lane("render", "渲染", percent, "headroom", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms` : "light load", "from-cyan-400/85 to-sky-300/35", buildSparkline(samples, 1), trend.label, trend.deltaLabel, [50, 85]);
}

function lane(
  id: string,
  label: string,
  percent: number,
  state: string,
  detail: string,
  toneClass: string,
  points: string,
  trendLabel: string,
  trendDelta: string,
  markerPercents: number[],
): HealthLane {
  return { id, label, percent, state, detail, toneClass, points, trendLabel, trendDelta, markerPercents };
}

function resolveTrend(values: number[], maxDomain: number): { label: string; deltaLabel: string } {
  const samples = values.filter((value) => Number.isFinite(value)).slice(-8);
  if (samples.length < 2 || !Number.isFinite(maxDomain) || maxDomain <= 0) {
    return { label: "steady", deltaLabel: "Δ —" };
  }
  const splitIndex = Math.max(1, Math.floor(samples.length / 2));
  const older = average(samples.slice(0, splitIndex));
  const newer = average(samples.slice(splitIndex));
  const delta = newer - older;
  const normalizedDelta = delta / maxDomain;
  if (Math.abs(normalizedDelta) < 0.04) {
    return { label: "steady", deltaLabel: `Δ ${formatSigned(delta)}` };
  }
  return {
    label: normalizedDelta > 0 ? "rising" : "falling",
    deltaLabel: `Δ ${formatSigned(delta)}`,
  };
}

function average(values: number[]): number {
  if (!values.length) return 0;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function formatSigned(value: number): string {
  const normalized = Object.is(value, -0) ? 0 : value;
  const prefix = normalized > 0 ? "+" : "";
  return `${prefix}${normalized.toFixed(2)}`;
}

function buildSparkline(values: number[], maxDomain: number): string {
  const width = 96;
  const height = 18;
  const samples = values.slice(-20);
  if (!samples.length || !Number.isFinite(maxDomain) || maxDomain <= 0) {
    return "";
  }
  return samples
    .map((value, index) => {
      const x = samples.length === 1 ? 0 : (index / (samples.length - 1)) * width;
      const normalized = Math.max(0, Math.min(1, (Number.isFinite(value) ? value : 0) / maxDomain));
      const y = height - normalized * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}
