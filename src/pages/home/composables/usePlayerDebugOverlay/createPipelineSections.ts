import type { MediaTelemetryPayload } from "@/modules/media-types";
import type { DebugRow, DebugSection } from "./types";
import {
  classifyAudioQueueState,
  classifyBudgetState,
  classifyGpuQueueState,
  classifyNetworkPressure,
  formatBytesPerSecond,
  isFiniteNumber,
  pushSnapshotRow,
  resolvePipelineBottleneck,
} from "./utils";

export function createPipelineSections(
  snapshot: Record<string, string>,
  telemetry: MediaTelemetryPayload | null,
): DebugSection[] {
  const ingressRows: DebugRow[] = [];
  const decodeRows: DebugRow[] = [];
  const audioRows: DebugRow[] = [];
  const renderRows: DebugRow[] = [];

  const networkRead = telemetry?.network_read_bytes_per_second;
  if (isFiniteNumber(networkRead)) ingressRows.push({ key: "pipe_network_read", label: "read", value: formatBytesPerSecond(networkRead) });
  const requiredRead = telemetry?.media_required_bytes_per_second;
  if (isFiniteNumber(requiredRead) && requiredRead > 0) ingressRows.push({ key: "pipe_network_required", label: "required", value: formatBytesPerSecond(requiredRead) });
  const sustainRatio = telemetry?.network_sustain_ratio;
  if (isFiniteNumber(sustainRatio)) {
    ingressRows.push({ key: "pipe_network_sustain", label: "sustain", value: `${sustainRatio.toFixed(2)}x` });
    ingressRows.push({ key: "pipe_network_state", label: "state", value: classifyNetworkPressure(sustainRatio) });
  }
  pushSnapshotRow(ingressRows, snapshot, "open", "source");
  pushSnapshotRow(ingressRows, snapshot, "video_demux", "demux");

  const sourceFps = telemetry?.source_fps;
  const frameBudgetMs = isFiniteNumber(sourceFps) && sourceFps > 0 ? 1000 / sourceFps : null;
  const stageCosts = telemetry?.video_stage_costs ?? null;
  if (isFiniteNumber(telemetry?.decode_avg_frame_cost_ms)) decodeRows.push({ key: "pipe_decode_avg", label: "total avg", value: `${telemetry.decode_avg_frame_cost_ms.toFixed(2)}ms` });
  if (isFiniteNumber(telemetry?.decode_max_frame_cost_ms)) decodeRows.push({ key: "pipe_decode_max", label: "total max", value: `${telemetry.decode_max_frame_cost_ms.toFixed(2)}ms` });
  if (isFiniteNumber(stageCosts?.receive_avg_ms)) decodeRows.push({ key: "pipe_decode_receive_avg", label: "receive", value: `${stageCosts.receive_avg_ms.toFixed(2)}ms` });
  if (isFiniteNumber(stageCosts?.hw_transfer_avg_ms)) decodeRows.push({ key: "pipe_decode_transfer_avg", label: "transfer", value: `${stageCosts.hw_transfer_avg_ms.toFixed(2)}ms` });
  if (isFiniteNumber(stageCosts?.scale_avg_ms)) decodeRows.push({ key: "pipe_decode_scale_avg", label: "scale/convert", value: `${stageCosts.scale_avg_ms.toFixed(2)}ms` });
  if (isFiniteNumber(stageCosts?.submit_avg_ms)) decodeRows.push({ key: "pipe_decode_submit_avg", label: "submit", value: `${stageCosts.submit_avg_ms.toFixed(2)}ms` });
  if (frameBudgetMs !== null && isFiniteNumber(stageCosts?.total_avg_ms)) {
    decodeRows.push({ key: "pipe_decode_budget", label: "frame budget", value: `${stageCosts.total_avg_ms.toFixed(2)} / ${frameBudgetMs.toFixed(2)}ms` });
    decodeRows.push({ key: "pipe_decode_state", label: "state", value: classifyBudgetState(stageCosts.total_avg_ms, frameBudgetMs) });
  } else if (frameBudgetMs !== null && isFiniteNumber(telemetry?.decode_avg_frame_cost_ms)) {
    decodeRows.push({ key: "pipe_decode_budget", label: "frame budget", value: `${telemetry.decode_avg_frame_cost_ms.toFixed(2)} / ${frameBudgetMs.toFixed(2)}ms` });
    decodeRows.push({ key: "pipe_decode_state", label: "state", value: classifyBudgetState(telemetry.decode_avg_frame_cost_ms, frameBudgetMs) });
  }
  if (typeof telemetry?.video_packet_soft_error_count === "number") {
    decodeRows.push({ key: "pipe_video_packet_soft_error_count", label: "packet err", value: String(telemetry.video_packet_soft_error_count) });
  }
  if (typeof telemetry?.video_frame_drop_count === "number") {
    decodeRows.push({ key: "pipe_video_frame_drop_count", label: "frame drops", value: String(telemetry.video_frame_drop_count) });
  }
  if (typeof telemetry?.video_scale_drop_count === "number") {
    decodeRows.push({ key: "pipe_video_scale_drop_count", label: "scale drops", value: String(telemetry.video_scale_drop_count) });
  }
  pushSnapshotRow(decodeRows, snapshot, "decoder_ready", "decoder");
  pushSnapshotRow(decodeRows, snapshot, "hw_decode_decision", "hw path");
  pushSnapshotRow(decodeRows, snapshot, "video_integrity", "integrity");

  const audioQueueDepth = telemetry?.audio_queue_depth_sources;
  if (typeof audioQueueDepth === "number") {
    audioRows.push({ key: "pipe_audio_queue", label: "queue", value: `${audioQueueDepth} buffers` });
    audioRows.push({ key: "pipe_audio_state", label: "state", value: classifyAudioQueueState(audioQueueDepth) });
  }
  if (isFiniteNumber(telemetry?.audio_drift_seconds)) {
    audioRows.push({ key: "pipe_audio_drift", label: "av drift", value: `${((telemetry.audio_drift_seconds ?? 0) * 1000).toFixed(2)}ms` });
  }
  pushSnapshotRow(audioRows, snapshot, "audio_pipeline_ready", "pipeline");
  pushSnapshotRow(audioRows, snapshot, "audio_output", "output");

  const gpuQueueDepth = telemetry?.gpu_queue_depth ?? telemetry?.queue_depth;
  const gpuQueueCapacity = telemetry?.gpu_queue_capacity ?? null;
  if (typeof gpuQueueDepth === "number") {
    renderRows.push({
      key: "pipe_gpu_queue",
      label: "gpu queue",
      value: gpuQueueCapacity && gpuQueueCapacity > 0 ? `${gpuQueueDepth}/${gpuQueueCapacity}` : String(gpuQueueDepth),
    });
    renderRows.push({ key: "pipe_gpu_state", label: "state", value: classifyGpuQueueState(gpuQueueDepth, gpuQueueCapacity) });
  }
  if (isFiniteNumber(telemetry?.video_submit_lead_ms)) {
    renderRows.push({ key: "pipe_submit_lead", label: "submit lead", value: `${telemetry.video_submit_lead_ms.toFixed(2)}ms` });
  }
  if (isFiniteNumber(telemetry?.render_estimated_cost_ms)) renderRows.push({ key: "pipe_render_cost", label: "render", value: `${telemetry.render_estimated_cost_ms.toFixed(2)}ms` });
  if (isFiniteNumber(telemetry?.render_present_lag_ms)) renderRows.push({ key: "pipe_present_lag", label: "present lag", value: `${telemetry.render_present_lag_ms.toFixed(2)}ms` });
  if (frameBudgetMs !== null && isFiniteNumber(telemetry?.render_estimated_cost_ms)) {
    renderRows.push({ key: "pipe_render_budget", label: "budget", value: `${telemetry.render_estimated_cost_ms.toFixed(2)} / ${frameBudgetMs.toFixed(2)}ms` });
    renderRows.push({ key: "pipe_render_state", label: "state", value: classifyBudgetState(telemetry.render_estimated_cost_ms, frameBudgetMs) });
  }
  pushSnapshotRow(renderRows, snapshot, "video_fps", "fps");

  return [
    { id: "ingress", title: "输入 / 拉流", rows: ingressRows },
    { id: "decode-pipe", title: "解码", rows: decodeRows },
    { id: "audio-pipe", title: "音频输出", rows: audioRows },
    { id: "render-pipe", title: "渲染", rows: renderRows },
    { id: "pipeline-summary", title: "瓶颈判断", rows: [{ key: "pipeline_bottleneck", label: "bottleneck", value: resolvePipelineBottleneck(telemetry ?? null) }] },
  ].filter((section) => section.rows.length > 0);
}
