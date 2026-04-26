import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import type { DebugRow, DebugSection } from "./types";
import {
  classifyAudioQueueState,
  classifyNetworkPressure,
  formatBytesPerSecond,
  formatHwModeLabel,
  isFiniteNumber,
  pushSnapshotRow,
  resolveHardwareCapabilityVerdict,
} from "./utils";

export function createOverviewSections(
  playback: PlaybackState | null,
  snapshot: Record<string, string>,
  telemetry: MediaTelemetryPayload | null,
): DebugSection[] {
  const sessionRows: DebugRow[] = [];
  const decodeRows: DebugRow[] = [];
  const transferRows: DebugRow[] = [];
  const capabilityRows: DebugRow[] = [];

  const status = playback?.status;
  if (status) sessionRows.push({ key: "status", label: "status", value: status });
  const positionSeconds = playback?.position_seconds;
  if (isFiniteNumber(positionSeconds)) {
    sessionRows.push({
      key: "position_seconds",
      label: "position",
      value: `${positionSeconds.toFixed(3)}s`,
    });
  }
  const sourceFps = telemetry?.source_fps;
  if (isFiniteNumber(sourceFps) && sourceFps > 0) {
    sessionRows.push({
      key: "source_fps",
      label: "source fps",
      value: `${sourceFps.toFixed(2)}fps`,
    });
  }
  if (typeof telemetry?.video_packet_soft_error_count === "number") {
    sessionRows.push({
      key: "video_packet_soft_error_count",
      label: "packet err",
      value: String(telemetry.video_packet_soft_error_count),
    });
  }
  if (typeof telemetry?.video_frame_drop_count === "number") {
    sessionRows.push({
      key: "video_frame_drop_count",
      label: "frame drops",
      value: String(telemetry.video_frame_drop_count),
    });
  }

  decodeRows.push({
    key: "decode_mode",
    label: "mode",
    value: formatHwModeLabel(playback?.hw_decode_mode || "auto"),
  });
  decodeRows.push({
    key: "decode_active",
    label: "active",
    value: playback?.hw_decode_active ? "hardware" : "software",
  });
  if (playback?.hw_decode_backend) {
    decodeRows.push({
      key: "hw_decode_backend",
      label: "backend",
      value: playback.hw_decode_backend,
    });
  }
  if (snapshot.hw_decode_decision) {
    decodeRows.push({
      key: "hw_decode_decision",
      label: "decision",
      value: snapshot.hw_decode_decision,
    });
  }
  if (snapshot.hw_decode_fallback) {
    decodeRows.push({
      key: "hw_decode_fallback",
      label: "fallback",
      value: snapshot.hw_decode_fallback,
    });
  }
  if (playback?.hw_decode_error) {
    decodeRows.push({
      key: "hw_decode_error",
      label: "reason",
      value: playback.hw_decode_error,
    });
  }

  capabilityRows.push({
    key: "hw_capability_summary",
    label: "verdict",
    value: resolveHardwareCapabilityVerdict(playback, snapshot),
  });
  pushSnapshotRow(capabilityRows, snapshot, "decoder_ready", "decoder");
  pushSnapshotRow(capabilityRows, snapshot, "video_format", "container");
  pushSnapshotRow(capabilityRows, snapshot, "video_codec_profile", "profile");

  const networkRead = telemetry?.network_read_bytes_per_second;
  if (isFiniteNumber(networkRead)) {
    transferRows.push({
      key: "network_read_bytes_per_second",
      label: "read",
      value: formatBytesPerSecond(networkRead),
    });
  }
  const requiredRead = telemetry?.media_required_bytes_per_second;
  if (isFiniteNumber(requiredRead) && requiredRead > 0) {
    transferRows.push({
      key: "media_required_bytes_per_second",
      label: "required",
      value: formatBytesPerSecond(requiredRead),
    });
  }
  const sustainRatio = telemetry?.network_sustain_ratio;
  if (isFiniteNumber(sustainRatio)) {
    transferRows.push({
      key: "network_sustain_ratio",
      label: "sustain",
      value: `${sustainRatio.toFixed(2)}x`,
    });
    transferRows.push({
      key: "network_pressure",
      label: "net pressure",
      value: classifyNetworkPressure(sustainRatio),
    });
  }
  const processCpu = telemetry?.process_cpu_percent;
  if (isFiniteNumber(processCpu)) {
    transferRows.push({
      key: "process_cpu_percent",
      label: "cpu",
      value: `${processCpu.toFixed(1)}%`,
    });
  }
  const processMemory = telemetry?.process_memory_mb;
  if (isFiniteNumber(processMemory)) {
    transferRows.push({
      key: "process_memory_mb",
      label: "memory",
      value: `${processMemory.toFixed(1)}MB`,
    });
  }
  const audioQueueDepth = telemetry?.audio_queue_depth_sources;
  if (typeof audioQueueDepth === "number") {
    transferRows.push({
      key: "audio_queue_depth_sources",
      label: "audio queue",
      value: `${audioQueueDepth} buffers`,
    });
    transferRows.push({
      key: "audio_queue_pressure",
      label: "audio state",
      value: classifyAudioQueueState(audioQueueDepth),
    });
  }

  return [
    { id: "session", title: "会话", rows: sessionRows },
    { id: "decode", title: "解码策略", rows: decodeRows },
    { id: "capability", title: "硬解能力判断", rows: capabilityRows },
    { id: "transfer", title: "传输与进程", rows: transferRows },
  ].filter((section) => section.rows.length > 0);
}
