import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import type { CurrentFrameSection, DebugRow } from "./types";
import { classifyGpuQueueState, isFiniteNumber } from "./utils";

export function createCurrentFrameSections(
  playback: PlaybackState | null,
  snapshot: Record<string, string>,
  telemetry: MediaTelemetryPayload | null,
): CurrentFrameSection[] {
  const timingRows: DebugRow[] = [];
  const outputRows: DebugRow[] = [];
  const decodeRows: DebugRow[] = [];

  const presentedVideoPts = telemetry?.current_presented_video_pts_seconds ?? telemetry?.current_video_pts_seconds;
  if (isFiniteNumber(presentedVideoPts)) timingRows.push({ key: "current_presented_video_pts_seconds", label: "presented pts", value: `${presentedVideoPts.toFixed(3)}s` });
  const submittedVideoPts = telemetry?.current_submitted_video_pts_seconds;
  if (isFiniteNumber(submittedVideoPts)) timingRows.push({ key: "current_submitted_video_pts_seconds", label: "submitted pts", value: `${submittedVideoPts.toFixed(3)}s` });
  const clockSeconds = telemetry?.clock_seconds;
  if (isFiniteNumber(clockSeconds)) timingRows.push({ key: "clock_seconds", label: "play clock", value: `${clockSeconds.toFixed(3)}s` });
  const audioClock = telemetry?.current_audio_clock_seconds;
  if (isFiniteNumber(audioClock)) timingRows.push({ key: "current_audio_clock_seconds", label: "audio clock", value: `${audioClock.toFixed(3)}s` });
  const driftSeconds = telemetry?.audio_drift_seconds;
  if (isFiniteNumber(driftSeconds)) timingRows.push({ key: "audio_drift_seconds", label: "av drift", value: `${(driftSeconds * 1000).toFixed(2)}ms` });
  const submitLeadMs = telemetry?.video_submit_lead_ms;
  if (isFiniteNumber(submitLeadMs)) timingRows.push({ key: "video_submit_lead_ms", label: "submit lead", value: `${submitLeadMs.toFixed(2)}ms` });
  const ptsGap = telemetry?.video_pts_gap_seconds;
  if (isFiniteNumber(ptsGap)) timingRows.push({ key: "video_pts_gap_seconds", label: "frame gap", value: `${(ptsGap * 1000).toFixed(2)}ms` });

  const frameType = telemetry?.current_frame_type?.trim();
  if (frameType) outputRows.push({ key: "current_frame_type", label: "frame type", value: frameType });
  const width = telemetry?.current_frame_width;
  const height = telemetry?.current_frame_height;
  if (isFiniteNumber(width) && isFiniteNumber(height) && width > 0 && height > 0) {
    outputRows.push({ key: "current_frame_size", label: "frame size", value: `${width}x${height}` });
  }
  const renderFps = telemetry?.render_fps;
  if (isFiniteNumber(renderFps)) outputRows.push({ key: "render_fps", label: "render fps", value: `${renderFps.toFixed(2)}fps` });

  const queueDepth = telemetry?.gpu_queue_depth ?? telemetry?.queue_depth;
  const queueCapacity = telemetry?.gpu_queue_capacity;
  if (typeof queueDepth === "number") {
    outputRows.push({
      key: "gpu_queue_depth",
      label: "gpu queue",
      value: typeof queueCapacity === "number" && queueCapacity > 0 ? `${queueDepth}/${queueCapacity}` : String(queueDepth),
    });
    outputRows.push({ key: "gpu_queue_pressure", label: "gpu state", value: classifyGpuQueueState(queueDepth, queueCapacity ?? null) });
  }
  const audioQueueDepth = telemetry?.audio_queue_depth_sources;
  if (typeof audioQueueDepth === "number") outputRows.push({ key: "audio_queue_depth", label: "audio queue", value: `${audioQueueDepth} buffers` });
  const playbackRate = telemetry?.playback_rate ?? playback?.playback_rate;
  if (isFiniteNumber(playbackRate)) outputRows.push({ key: "playback_rate", label: "rate", value: `${playbackRate.toFixed(2)}x` });

  const renderLag = telemetry?.render_present_lag_ms;
  if (isFiniteNumber(renderLag)) decodeRows.push({ key: "render_present_lag_ms", label: "present lag", value: `${renderLag.toFixed(2)}ms` });
  const decodeAvg = telemetry?.decode_avg_frame_cost_ms;
  if (isFiniteNumber(decodeAvg)) decodeRows.push({ key: "decode_avg_frame_cost_ms", label: "decode avg", value: `${decodeAvg.toFixed(2)}ms` });
  const decodeMax = telemetry?.decode_max_frame_cost_ms;
  if (isFiniteNumber(decodeMax)) decodeRows.push({ key: "decode_max_frame_cost_ms", label: "decode max", value: `${decodeMax.toFixed(2)}ms` });
  const decodeSamples = telemetry?.decode_samples;
  if (typeof decodeSamples === "number" && decodeSamples > 0) decodeRows.push({ key: "decode_samples", label: "window", value: `${decodeSamples} frames` });
  const packetSoftErrors = telemetry?.video_packet_soft_error_count;
  if (typeof packetSoftErrors === "number") decodeRows.push({ key: "video_packet_soft_error_count", label: "packet soft err", value: String(packetSoftErrors) });
  const frameDrops = telemetry?.video_frame_drop_count;
  if (typeof frameDrops === "number") decodeRows.push({ key: "video_frame_drop_count", label: "frame drops", value: String(frameDrops) });
  const integrity = snapshot.video_integrity;
  if (integrity) decodeRows.push({ key: "video_integrity", label: "integrity", value: integrity });

  return [
    { id: "timing", title: "帧时序", rows: timingRows },
    { id: "output", title: "输出状态", rows: outputRows },
    { id: "decode", title: "解码/呈现", rows: decodeRows },
  ].filter((section) => section.rows.length > 0);
}
