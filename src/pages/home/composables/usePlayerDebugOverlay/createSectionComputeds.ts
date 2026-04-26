import { computed, type Ref } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import type { CurrentFrameSection, DebugRow, DebugSection } from "./types";
import {
  classifyAudioQueueState,
  classifyBudgetState,
  classifyGpuQueueState,
  classifyNetworkPressure,
  formatBytesPerSecond,
  formatHwModeLabel,
  isFiniteNumber,
  pushSnapshotRow,
  resolveHardwareCapabilityVerdict,
  resolvePipelineBottleneck,
} from "./utils";

export function createSectionComputeds(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
  latestTelemetry?: Ref<MediaTelemetryPayload | null>,
) {
  const overviewSections = computed((): DebugSection[] => {
    const telemetry = latestTelemetry?.value;
    const snapshot = debugSnapshot.value;
    const sessionRows: DebugRow[] = [];
    const decodeRows: DebugRow[] = [];
    const transferRows: DebugRow[] = [];
    const capabilityRows: DebugRow[] = [];

    const status = playback.value?.status;
    if (status) {
      sessionRows.push({ key: "status", label: "status", value: status });
    }
    const positionSeconds = playback.value?.position_seconds;
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

    decodeRows.push({
      key: "decode_mode",
      label: "mode",
      value: formatHwModeLabel(playback.value?.hw_decode_mode || "auto"),
    });
    decodeRows.push({
      key: "decode_active",
      label: "active",
      value: playback.value?.hw_decode_active ? "hardware" : "software",
    });
    if (playback.value?.hw_decode_backend) {
      decodeRows.push({
        key: "hw_decode_backend",
        label: "backend",
        value: playback.value.hw_decode_backend,
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
    if (playback.value?.hw_decode_error) {
      decodeRows.push({
        key: "hw_decode_error",
        label: "reason",
        value: playback.value.hw_decode_error,
      });
    }

    capabilityRows.push({
      key: "hw_capability_summary",
      label: "verdict",
      value: resolveHardwareCapabilityVerdict(playback.value, snapshot),
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
  });

  const streamSections = computed((): DebugSection[] => {
    const snapshot = debugSnapshot.value;
    const inputRows: DebugRow[] = [];
    const videoRows: DebugRow[] = [];
    const audioRows: DebugRow[] = [];

    pushSnapshotRow(inputRows, snapshot, "open", "source");
    pushSnapshotRow(inputRows, snapshot, "video_demux", "demux");
    pushSnapshotRow(inputRows, snapshot, "video_gop", "gop");

    pushSnapshotRow(videoRows, snapshot, "video_format", "container");
    pushSnapshotRow(videoRows, snapshot, "video_codec_profile", "codec");
    pushSnapshotRow(videoRows, snapshot, "video_stream", "stream");
    pushSnapshotRow(videoRows, snapshot, "video_frame_format", "frame fmt");
    pushSnapshotRow(videoRows, snapshot, "decoder_ready", "decoder");
    pushSnapshotRow(videoRows, snapshot, "color_profile", "color");

    pushSnapshotRow(audioRows, snapshot, "audio", "stream");
    pushSnapshotRow(audioRows, snapshot, "audio_format", "format");
    pushSnapshotRow(audioRows, snapshot, "audio_pipeline_ready", "pipeline");
    pushSnapshotRow(audioRows, snapshot, "audio_output", "output");

    return [
      { id: "input", title: "输入与探测", rows: inputRows },
      { id: "video-chain", title: "视频链路", rows: videoRows },
      { id: "audio-chain", title: "音频链路", rows: audioRows },
    ].filter((section) => section.rows.length > 0);
  });

  const timingSections = computed((): DebugSection[] => {
    const telemetry = latestTelemetry?.value;
    const snapshot = debugSnapshot.value;
    const syncRows: DebugRow[] = [];
    const cadenceRows: DebugRow[] = [];
    const perfRows: DebugRow[] = [];

    pushSnapshotRow(syncRows, snapshot, "av_sync", "av sync");
    const videoTimestampStats = telemetry?.video_timestamps;
    if (videoTimestampStats) {
      syncRows.push({ key: "video_ts_samples", label: "ts samples", value: String(videoTimestampStats.samples) });
      syncRows.push({
        key: "video_ts_missing",
        label: "pts missing",
        value: `${videoTimestampStats.pts_missing_ratio_percent.toFixed(2)}%`,
      });
      syncRows.push({
        key: "video_ts_backtrack",
        label: "backtrack",
        value: String(videoTimestampStats.pts_backtrack_count),
      });
      syncRows.push({
        key: "video_ts_jitter_avg",
        label: "jitter avg",
        value: `${videoTimestampStats.jitter_avg_ms.toFixed(3)}ms`,
      });
      syncRows.push({
        key: "video_ts_jitter_max",
        label: "jitter max",
        value: `${videoTimestampStats.jitter_max_ms.toFixed(3)}ms`,
      });
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
      cadenceRows.push({
        key: "frame_budget_ms",
        label: "frame budget",
        value: `${(1000 / telemetry.source_fps).toFixed(2)}ms`,
      });
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
      perfRows.push({
        key: "decode_headroom_ms",
        label: "decode room",
        value: `${(budgetMs - (telemetry.decode_avg_frame_cost_ms ?? 0)).toFixed(2)}ms`,
      });
    }
    if (isFiniteNumber(telemetry?.render_estimated_cost_ms) && isFiniteNumber(telemetry?.source_fps) && telemetry.source_fps > 0) {
      const budgetMs = 1000 / telemetry.source_fps;
      perfRows.push({
        key: "render_headroom_ms",
        label: "render room",
        value: `${(budgetMs - (telemetry.render_estimated_cost_ms ?? 0)).toFixed(2)}ms`,
      });
    }
    return [
      { id: "sync", title: "同步质量", rows: syncRows },
      { id: "cadence", title: "节奏与帧型", rows: cadenceRows },
      { id: "perf", title: "预算与性能", rows: perfRows },
    ].filter((section) => section.rows.length > 0);
  });

  const pipelineSections = computed((): DebugSection[] => {
    const telemetry = latestTelemetry?.value;
    const snapshot = debugSnapshot.value;
    const ingressRows: DebugRow[] = [];
    const decodeRows: DebugRow[] = [];
    const audioRows: DebugRow[] = [];
    const renderRows: DebugRow[] = [];

    const networkRead = telemetry?.network_read_bytes_per_second;
    if (isFiniteNumber(networkRead)) {
      ingressRows.push({ key: "pipe_network_read", label: "read", value: formatBytesPerSecond(networkRead) });
    }
    const requiredRead = telemetry?.media_required_bytes_per_second;
    if (isFiniteNumber(requiredRead) && requiredRead > 0) {
      ingressRows.push({ key: "pipe_network_required", label: "required", value: formatBytesPerSecond(requiredRead) });
    }
    const sustainRatio = telemetry?.network_sustain_ratio;
    if (isFiniteNumber(sustainRatio)) {
      ingressRows.push({ key: "pipe_network_sustain", label: "sustain", value: `${sustainRatio.toFixed(2)}x` });
      ingressRows.push({ key: "pipe_network_state", label: "state", value: classifyNetworkPressure(sustainRatio) });
    }
    pushSnapshotRow(ingressRows, snapshot, "open", "source");
    pushSnapshotRow(ingressRows, snapshot, "video_demux", "demux");

    const sourceFps = telemetry?.source_fps;
    const frameBudgetMs = isFiniteNumber(sourceFps) && sourceFps > 0 ? 1000 / sourceFps : null;
    if (isFiniteNumber(telemetry?.decode_avg_frame_cost_ms)) {
      decodeRows.push({ key: "pipe_decode_avg", label: "avg", value: `${telemetry.decode_avg_frame_cost_ms.toFixed(2)}ms` });
    }
    if (isFiniteNumber(telemetry?.decode_max_frame_cost_ms)) {
      decodeRows.push({ key: "pipe_decode_max", label: "max", value: `${telemetry.decode_max_frame_cost_ms.toFixed(2)}ms` });
    }
    if (frameBudgetMs !== null && isFiniteNumber(telemetry?.decode_avg_frame_cost_ms)) {
      decodeRows.push({
        key: "pipe_decode_budget",
        label: "budget",
        value: `${telemetry.decode_avg_frame_cost_ms.toFixed(2)} / ${frameBudgetMs.toFixed(2)}ms`,
      });
      decodeRows.push({
        key: "pipe_decode_state",
        label: "state",
        value: classifyBudgetState(telemetry.decode_avg_frame_cost_ms, frameBudgetMs),
      });
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
      audioRows.push({
        key: "pipe_audio_drift",
        label: "av drift",
        value: `${((telemetry.audio_drift_seconds ?? 0) * 1000).toFixed(2)}ms`,
      });
    }
    pushSnapshotRow(audioRows, snapshot, "audio_pipeline_ready", "pipeline");
    pushSnapshotRow(audioRows, snapshot, "audio_output", "output");

    const gpuQueueDepth = telemetry?.gpu_queue_depth ?? telemetry?.queue_depth;
    const gpuQueueCapacity = telemetry?.gpu_queue_capacity ?? null;
    if (typeof gpuQueueDepth === "number") {
      renderRows.push({
        key: "pipe_gpu_queue",
        label: "gpu queue",
        value: gpuQueueCapacity && gpuQueueCapacity > 0
          ? `${gpuQueueDepth}/${gpuQueueCapacity}`
          : String(gpuQueueDepth),
      });
      renderRows.push({
        key: "pipe_gpu_state",
        label: "state",
        value: classifyGpuQueueState(gpuQueueDepth, gpuQueueCapacity),
      });
    }
    if (isFiniteNumber(telemetry?.render_estimated_cost_ms)) {
      renderRows.push({ key: "pipe_render_cost", label: "render", value: `${telemetry.render_estimated_cost_ms.toFixed(2)}ms` });
    }
    if (isFiniteNumber(telemetry?.render_present_lag_ms)) {
      renderRows.push({ key: "pipe_present_lag", label: "present lag", value: `${telemetry.render_present_lag_ms.toFixed(2)}ms` });
    }
    if (frameBudgetMs !== null && isFiniteNumber(telemetry?.render_estimated_cost_ms)) {
      renderRows.push({
        key: "pipe_render_budget",
        label: "budget",
        value: `${telemetry.render_estimated_cost_ms.toFixed(2)} / ${frameBudgetMs.toFixed(2)}ms`,
      });
      renderRows.push({
        key: "pipe_render_state",
        label: "state",
        value: classifyBudgetState(telemetry.render_estimated_cost_ms, frameBudgetMs),
      });
    }
    pushSnapshotRow(renderRows, snapshot, "video_fps", "fps");

    return [
      { id: "ingress", title: "输入 / 拉流", rows: ingressRows },
      { id: "decode-pipe", title: "解码", rows: decodeRows },
      { id: "audio-pipe", title: "音频输出", rows: audioRows },
      { id: "render-pipe", title: "渲染", rows: renderRows },
      {
        id: "pipeline-summary",
        title: "瓶颈判断",
        rows: [{ key: "pipeline_bottleneck", label: "bottleneck", value: resolvePipelineBottleneck(telemetry ?? null) }],
      },
    ].filter((section) => section.rows.length > 0);
  });

  const currentFrameSections = computed((): CurrentFrameSection[] => {
    const telemetry = latestTelemetry?.value;
    const snapshot = debugSnapshot.value;
    const timingRows: DebugRow[] = [];
    const outputRows: DebugRow[] = [];
    const decodeRows: DebugRow[] = [];

    const videoPts = telemetry?.current_video_pts_seconds;
    if (isFiniteNumber(videoPts)) {
      timingRows.push({ key: "current_video_pts_seconds", label: "video pts", value: `${videoPts.toFixed(3)}s` });
    }

    const clockSeconds = telemetry?.clock_seconds;
    if (isFiniteNumber(clockSeconds)) {
      timingRows.push({ key: "clock_seconds", label: "play clock", value: `${clockSeconds.toFixed(3)}s` });
    }

    const audioClock = telemetry?.current_audio_clock_seconds;
    if (isFiniteNumber(audioClock)) {
      timingRows.push({ key: "current_audio_clock_seconds", label: "audio clock", value: `${audioClock.toFixed(3)}s` });
    }

    const driftSeconds = telemetry?.audio_drift_seconds;
    if (isFiniteNumber(driftSeconds)) {
      timingRows.push({ key: "audio_drift_seconds", label: "av drift", value: `${(driftSeconds * 1000).toFixed(2)}ms` });
    }

    const ptsGap = telemetry?.video_pts_gap_seconds;
    if (isFiniteNumber(ptsGap)) {
      timingRows.push({ key: "video_pts_gap_seconds", label: "frame gap", value: `${(ptsGap * 1000).toFixed(2)}ms` });
    }

    const frameType = telemetry?.current_frame_type?.trim();
    if (frameType) {
      outputRows.push({ key: "current_frame_type", label: "frame type", value: frameType });
    }

    const width = telemetry?.current_frame_width;
    const height = telemetry?.current_frame_height;
    if (isFiniteNumber(width) && isFiniteNumber(height) && width > 0 && height > 0) {
      outputRows.push({ key: "current_frame_size", label: "frame size", value: `${width}x${height}` });
    }

    const renderFps = telemetry?.render_fps;
    if (isFiniteNumber(renderFps)) {
      outputRows.push({ key: "render_fps", label: "render fps", value: `${renderFps.toFixed(2)}fps` });
    }

    const queueDepth = telemetry?.gpu_queue_depth ?? telemetry?.queue_depth;
    const queueCapacity = telemetry?.gpu_queue_capacity;
    if (typeof queueDepth === "number") {
      outputRows.push({
        key: "gpu_queue_depth",
        label: "gpu queue",
        value: typeof queueCapacity === "number" && queueCapacity > 0
          ? `${queueDepth}/${queueCapacity}`
          : String(queueDepth),
      });
      outputRows.push({
        key: "gpu_queue_pressure",
        label: "gpu state",
        value: classifyGpuQueueState(queueDepth, queueCapacity ?? null),
      });
    }

    const audioQueueDepth = telemetry?.audio_queue_depth_sources;
    if (typeof audioQueueDepth === "number") {
      outputRows.push({ key: "audio_queue_depth", label: "audio queue", value: `${audioQueueDepth} buffers` });
    }

    const playbackRate = telemetry?.playback_rate ?? playback.value?.playback_rate;
    if (isFiniteNumber(playbackRate)) {
      outputRows.push({ key: "playback_rate", label: "rate", value: `${playbackRate.toFixed(2)}x` });
    }

    const renderLag = telemetry?.render_present_lag_ms;
    if (isFiniteNumber(renderLag)) {
      decodeRows.push({ key: "render_present_lag_ms", label: "present lag", value: `${renderLag.toFixed(2)}ms` });
    }

    const decodeAvg = telemetry?.decode_avg_frame_cost_ms;
    if (isFiniteNumber(decodeAvg)) {
      decodeRows.push({ key: "decode_avg_frame_cost_ms", label: "decode avg", value: `${decodeAvg.toFixed(2)}ms` });
    }

    const decodeMax = telemetry?.decode_max_frame_cost_ms;
    if (isFiniteNumber(decodeMax)) {
      decodeRows.push({ key: "decode_max_frame_cost_ms", label: "decode max", value: `${decodeMax.toFixed(2)}ms` });
    }

    const decodeSamples = telemetry?.decode_samples;
    if (typeof decodeSamples === "number" && decodeSamples > 0) {
      decodeRows.push({ key: "decode_samples", label: "window", value: `${decodeSamples} frames` });
    }

    const integrity = snapshot.video_integrity;
    if (integrity) {
      decodeRows.push({ key: "video_integrity", label: "integrity", value: integrity });
    }

    return [
      { id: "timing", title: "帧时序", rows: timingRows },
      { id: "output", title: "输出状态", rows: outputRows },
      { id: "decode", title: "解码/呈现", rows: decodeRows },
    ].filter((section) => section.rows.length > 0);
  });

  return {
    overviewSections,
    streamSections,
    timingSections,
    pipelineSections,
    currentFrameSections,
  };
}
