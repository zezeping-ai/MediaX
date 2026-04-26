import type { MediaTelemetryPayload } from "@/modules/media-types";
import type { DebugRow, DebugSection } from "./types";
import { isFiniteNumber, pushSnapshotRow } from "./utils";

export function createTimingSections(
  snapshot: Record<string, string>,
  telemetry: MediaTelemetryPayload | null,
): DebugSection[] {
  const syncRows: DebugRow[] = [];
  const cadenceRows: DebugRow[] = [];
  const perfRows: DebugRow[] = [];

  pushSnapshotRow(syncRows, snapshot, "av_sync", "av sync");
  const videoTimestampStats = telemetry?.video_timestamps;
  if (videoTimestampStats) {
    syncRows.push({ key: "video_ts_samples", label: "ts samples", value: String(videoTimestampStats.samples) });
    syncRows.push({ key: "video_ts_missing", label: "pts missing", value: `${videoTimestampStats.pts_missing_ratio_percent.toFixed(2)}%` });
    syncRows.push({ key: "video_ts_backtrack", label: "backtrack", value: String(videoTimestampStats.pts_backtrack_count) });
    syncRows.push({ key: "video_ts_jitter_avg", label: "jitter avg", value: `${videoTimestampStats.jitter_avg_ms.toFixed(3)}ms` });
    syncRows.push({ key: "video_ts_jitter_max", label: "jitter max", value: `${videoTimestampStats.jitter_max_ms.toFixed(3)}ms` });
  } else {
    pushSnapshotRow(syncRows, snapshot, "video_timestamps", "timestamps");
  }
  if (isFiniteNumber(telemetry?.audio_drift_seconds)) {
    syncRows.push({
      key: "audio_drift_seconds_window",
      label: "drift now",
      value: `${((telemetry?.audio_drift_seconds ?? 0) * 1000).toFixed(2)}ms`,
    });
  }

  pushSnapshotRow(cadenceRows, snapshot, "video_fps", "render fps");
  pushSnapshotRow(cadenceRows, snapshot, "video_gap", "gap");
  const frameTypeStats = telemetry?.frame_types;
  if (frameTypeStats) {
    cadenceRows.push({ key: "frame_type_i_ratio", label: "I ratio", value: `${frameTypeStats.i_ratio_percent.toFixed(1)}%` });
    cadenceRows.push({ key: "frame_type_p_ratio", label: "P ratio", value: `${frameTypeStats.p_ratio_percent.toFixed(1)}%` });
    cadenceRows.push({ key: "frame_type_b_ratio", label: "B ratio", value: `${frameTypeStats.b_ratio_percent.toFixed(1)}%` });
    cadenceRows.push({ key: "frame_type_other_ratio", label: "other", value: `${frameTypeStats.other_ratio_percent.toFixed(1)}%` });
    cadenceRows.push({ key: "frame_type_samples", label: "samples", value: String(frameTypeStats.sample_count) });
  } else {
    pushSnapshotRow(cadenceRows, snapshot, "video_frame_types", "frame mix");
  }
  if (isFiniteNumber(telemetry?.source_fps) && telemetry.source_fps > 0) {
    cadenceRows.push({ key: "frame_budget_ms", label: "frame budget", value: `${(1000 / telemetry.source_fps).toFixed(2)}ms` });
  }

  const decodeQuantiles = telemetry?.decode_quantiles;
  if (decodeQuantiles) {
    perfRows.push({ key: "decode_p50_ms", label: "decode p50", value: `${decodeQuantiles.p50_ms.toFixed(3)}ms` });
    perfRows.push({ key: "decode_p95_ms", label: "decode p95", value: `${decodeQuantiles.p95_ms.toFixed(3)}ms` });
    perfRows.push({ key: "decode_p99_ms", label: "decode p99", value: `${decodeQuantiles.p99_ms.toFixed(3)}ms` });
    perfRows.push({ key: "decode_window_samples", label: "decode win", value: String(decodeQuantiles.sample_count) });
  } else {
    pushSnapshotRow(perfRows, snapshot, "decode_cost_quantiles", "decode");
  }
  pushSnapshotRow(perfRows, snapshot, "telemetry_render", "render");
  pushSnapshotRow(perfRows, snapshot, "telemetry_resources", "resources");
  if (isFiniteNumber(telemetry?.decode_avg_frame_cost_ms) && isFiniteNumber(telemetry?.source_fps) && telemetry.source_fps > 0) {
    const budgetMs = 1000 / telemetry.source_fps;
    perfRows.push({ key: "decode_headroom_ms", label: "decode room", value: `${(budgetMs - (telemetry.decode_avg_frame_cost_ms ?? 0)).toFixed(2)}ms` });
  }
  if (isFiniteNumber(telemetry?.render_estimated_cost_ms) && isFiniteNumber(telemetry?.source_fps) && telemetry.source_fps > 0) {
    const budgetMs = 1000 / telemetry.source_fps;
    perfRows.push({ key: "render_headroom_ms", label: "render room", value: `${(budgetMs - (telemetry.render_estimated_cost_ms ?? 0)).toFixed(2)}ms` });
  }

  return [
    { id: "sync", title: "同步质量", rows: syncRows },
    { id: "cadence", title: "节奏与帧型", rows: cadenceRows },
    { id: "perf", title: "预算与性能", rows: perfRows },
  ].filter((section) => section.rows.length > 0);
}
