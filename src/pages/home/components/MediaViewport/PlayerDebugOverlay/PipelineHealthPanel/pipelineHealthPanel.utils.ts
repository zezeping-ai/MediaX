import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import type { HealthLane } from "./pipelineHealthPanel.types";

export function buildPipelineHealthLanes(
  source: string,
  playback: PlaybackState | null,
  telemetry: MediaTelemetryPayload | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane[] {
  if (!telemetry) {
    return [];
  }
  const sourceFps = telemetry.source_fps > 0 ? telemetry.source_fps : 0;
  const frameBudgetMs = sourceFps > 0 ? 1000 / sourceFps : null;
  const networkRatio = telemetry.network_sustain_ratio ?? null;
  const renderCost = telemetry.render_estimated_cost_ms ?? null;
  const audioQueue = telemetry.audio_queue_depth_sources ?? null;
  const stageCosts = telemetry.video_stage_costs ?? null;

  return [
    buildSourceSupplyLane(source, telemetry, networkRatio, history),
    buildStageLane(
      "decode_receive",
      "解码接收",
      stageCosts?.receive_avg_ms ?? null,
      frameBudgetMs,
      history,
      (item) => item.telemetry.video_stage_costs?.receive_avg_ms ?? null,
      () => "packet -> frame",
    ),
    buildStageLane(
      "hw_transfer",
      playback?.hw_decode_active ? "帧传输 / HW" : "帧传输 / SW",
      stageCosts?.hw_transfer_avg_ms ?? null,
      frameBudgetMs,
      history,
      (item) => item.telemetry.video_stage_costs?.hw_transfer_avg_ms ?? null,
      () => (playback?.hw_decode_active ? "hw frame -> cpu" : "software passthrough"),
    ),
    buildStageLane(
      "scale_convert",
      "缩放 / 转换",
      stageCosts?.scale_avg_ms ?? null,
      frameBudgetMs,
      history,
      (item) => item.telemetry.video_stage_costs?.scale_avg_ms ?? null,
      () => "colorspace / resize",
    ),
    buildAudioLane(audioQueue, history),
    buildStageLane(
      "submit",
      "提交",
      stageCosts?.submit_avg_ms ?? null,
      frameBudgetMs,
      history,
      (item) => item.telemetry.video_stage_costs?.submit_avg_ms ?? null,
      () => "submit to renderer",
    ),
    buildRenderLane(telemetry.gpu_queue_utilization ?? null, renderCost, frameBudgetMs, history),
  ];
}

function buildSourceSupplyLane(
  source: string,
  telemetry: MediaTelemetryPayload,
  ratio: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  const label = resolveSourceSupplyLabel(source);
  const normalized = ratio && Number.isFinite(ratio) ? Math.max(0, Math.min(1.2, ratio)) : 0;
  const percent = Math.min(100, normalized * 100);
  const rawSamples = history.map((item) => item.telemetry.network_sustain_ratio ?? 0);
  const trend = resolveTrend(rawSamples, 1.2);
  const samples = buildPercentSamples(rawSamples, 1.2, normalized);
  const detail = formatSupplyDetail(telemetry, ratio);
  if (ratio === null || !Number.isFinite(ratio)) {
    return lane("source_supply", label, 0, "not sampled", "等待输入供给采样", "unknown", "pending", "—", [83.33, 95.83], []);
  }
  if (ratio >= 1.15) {
    return lane("source_supply", label, percent, "ample headroom", detail, "healthy", trend.label, trend.deltaLabel, [83.33, 95.83], samples);
  }
  if (ratio >= 1.0) {
    return lane("source_supply", label, percent, "supply matched", detail, "tight", trend.label, trend.deltaLabel, [83.33, 95.83], samples);
  }
  return lane("source_supply", label, percent, "supply risk", detail, "risk", trend.label, trend.deltaLabel, [83.33, 95.83], samples);
}

function buildStageLane(
  id: string,
  label: string,
  costMs: number | null,
  budgetMs: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
  resolveHistoryCost: (item: { at_ms: number; telemetry: MediaTelemetryPayload }) => number | null,
  resolveSemanticHint: () => string,
): HealthLane {
  if (costMs === null || budgetMs === null || !Number.isFinite(costMs) || !Number.isFinite(budgetMs) || budgetMs <= 0) {
    return lane(id, label, 0, "not sampled", `等待${label}采样`, "unknown", "pending", "—", [50, 85], []);
  }
  const ratio = costMs / budgetMs;
  const percent = Math.min(100, Math.max(0, ratio * 100));
  const rawSamples = history.map((item) => {
    const fps = item.telemetry.source_fps > 0 ? item.telemetry.source_fps : 0;
    const budget = fps > 0 ? 1000 / fps : null;
    const cost = resolveHistoryCost(item);
    return budget && cost !== null && Number.isFinite(cost) ? cost / budget : 0;
  });
  const trend = resolveTrend(rawSamples, 1);
  const samples = buildPercentSamples(rawSamples, 1, ratio);
  const detail = `${costMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms · ${resolveSemanticHint()}`;
  if (ratio >= 1) {
    return lane(id, label, percent, "over frame budget", detail, "risk", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  if (ratio >= 0.85) {
    return lane(id, label, percent, "near frame budget", detail, "tight", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  if (ratio >= 0.5) {
    return lane(id, label, percent, "within frame budget", detail, "healthy", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  return lane(id, label, percent, "ample headroom", detail, "headroom", trend.label, trend.deltaLabel, [50, 85], samples);
}

function buildAudioLane(
  queueDepth: number | null,
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>,
): HealthLane {
  if (queueDepth === null || !Number.isFinite(queueDepth)) {
    return lane("audio", "音频", 0, "unknown", "等待音频队列", "unknown", "pending", "—", [25, 66.67], []);
  }
  const percent = Math.min(100, Math.max(8, (queueDepth / 12) * 100));
  const rawSamples = history.map((item) => item.telemetry.audio_queue_depth_sources ?? 0);
  const trend = resolveTrend(rawSamples, 12);
  const samples = buildPercentSamples(rawSamples, 12, queueDepth);
  if (queueDepth < 3) {
    return lane("audio", "音频", percent, "low buffer", `${queueDepth} buffers`, "risk", trend.label, trend.deltaLabel, [25, 66.67], samples);
  }
  if (queueDepth < 8) {
    return lane("audio", "音频", percent, "stable", `${queueDepth} buffers`, "healthy", trend.label, trend.deltaLabel, [25, 66.67], samples);
  }
  return lane("audio", "音频", percent, "deep buffer", `${queueDepth} buffers`, "headroom", trend.label, trend.deltaLabel, [25, 66.67], samples);
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
  const rawSamples = history.map((item) => {
    const fps = item.telemetry.source_fps > 0 ? item.telemetry.source_fps : 0;
    const budget = fps > 0 ? 1000 / fps : null;
    const queue = item.telemetry.gpu_queue_utilization ?? 0;
    const cost = item.telemetry.render_estimated_cost_ms;
    const budgetRatio = budget && cost !== null && Number.isFinite(cost) ? cost / budget : 0;
    return Math.max(queue, budgetRatio);
  });
  const trend = resolveTrend(rawSamples, 1);
  const samples = buildPercentSamples(rawSamples, 1, ratio);
  if (ratio >= 1 || queueRatio >= 0.85) {
    return lane("render_queue", "渲染队列", percent, "backpressure", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms · queue saturated` : "queue saturated", "risk", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  if (ratio >= 0.85) {
    return lane("render_queue", "渲染队列", percent, "tight", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms · near budget` : "near budget", "tight", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  if (ratio >= 0.5) {
    return lane("render_queue", "渲染队列", percent, "healthy", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms · steady` : "steady", "healthy", trend.label, trend.deltaLabel, [50, 85], samples);
  }
  return lane("render_queue", "渲染队列", percent, "headroom", renderCostMs !== null && budgetMs !== null ? `${renderCostMs.toFixed(2)} / ${budgetMs.toFixed(2)}ms · light load` : "light load", "headroom", trend.label, trend.deltaLabel, [50, 85], samples);
}

function lane(
  id: string,
  label: string,
  percent: number,
  state: string,
  detail: string,
  tone: HealthLane["tone"],
  trendLabel: string,
  trendDelta: string,
  markerPercents: number[],
  samples: number[],
): HealthLane {
  return { id, label, percent, state, detail, tone, trendLabel, trendDelta, markerPercents, samples };
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

function resolveSourceSupplyLabel(source: string) {
  return isRemoteSource(source) ? "输入供给 / URL" : "输入供给 / 本地";
}

function isRemoteSource(source: string) {
  return /^https?:\/\//i.test(source);
}

function formatSupplyDetail(telemetry: MediaTelemetryPayload, ratio: number | null) {
  const read = telemetry.network_read_bytes_per_second;
  const required = telemetry.media_required_bytes_per_second;
  if (
    ratio !== null &&
    Number.isFinite(ratio) &&
    read !== null &&
    read !== undefined &&
    required !== null &&
    required !== undefined &&
    Number.isFinite(read) &&
    Number.isFinite(required)
  ) {
    return `feed ${ratio.toFixed(2)}x · ${formatBytesPerSecond(read)} / ${formatBytesPerSecond(required)}`;
  }
  if (ratio !== null && Number.isFinite(ratio)) {
    return `feed ${ratio.toFixed(2)}x`;
  }
  return "feed n/a";
}

function formatBytesPerSecond(value: number) {
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let size = Math.max(0, value);
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex += 1;
  }
  const digits = size >= 100 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(digits)} ${units[unitIndex]}`;
}

function buildPercentSamples(values: number[], maxDomain: number, currentValue: number): number[] {
  const sampleCount = 24;
  const historySamples = values
    .filter((value) => Number.isFinite(value))
    .slice(-(sampleCount - 1))
    .map((value) => normalizedPercent(value, maxDomain));
  const currentPercent = normalizedPercent(currentValue, maxDomain);
  return [...historySamples, currentPercent].slice(-sampleCount);
}

function normalizedPercent(value: number, maxDomain: number) {
  if (!Number.isFinite(value) || !Number.isFinite(maxDomain) || maxDomain <= 0) {
    return 0;
  }
  return Math.max(0, Math.min(100, (value / maxDomain) * 100));
}
