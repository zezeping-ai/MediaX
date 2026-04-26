import { computed, type Ref } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";

const PREFERRED_DEBUG_ORDER = [
  "open",
  "metadata_ready",
  "audio_pipeline_ready",
  "video_demux",
  "video_gop",
  "decoder_ready",
  "hw_decode_decision",
  "running",
  "first_frame",
  "color_profile",
  "video_integrity",
  "video_pipeline",
  "video_fps",
  "video_gap",
  "video_timestamps",
  "video_frame_types",
  "decode_cost_quantiles",
  "audio_stats",
  "audio_output",
  "av_sync",
  "audio_resume",
  "seek",
  "telemetry_timing",
  "telemetry_render",
  "telemetry_resources",
  "decode_error",
  "hw_decode_fallback",
] as const;

const DEBUG_LABELS: Record<string, string> = {
  open: "打开",
  stream_start: "流启动",
  metadata_ready: "元数据就绪",
  audio_pipeline_ready: "音频管线就绪",
  decoder_ready: "解码器就绪",
  hw_decode_decision: "硬解决策",
  video_format: "视频格式",
  color_profile: "色彩配置",
  color_profill: "色彩配置",
  video_stream: "视频流",
  video_demux: "视频解复用",
  video_gop: "GOP/场景切换",
  video_timestamps: "时间戳质量",
  video_frame_types: "帧类型分布",
  decode_cost_quantiles: "耗时分位数",
  audio: "音频流",
  running: "播放状态",
  first_frame: "首帧就绪",
  video_pipeline: "视频管线",
  video_integrity: "视频完整性",
  telemetry_timing: "时序性能",
  telemetry_resources: "进程资源",
  telemetry_render: "渲染性能",
  video_fps: "视频帧率",
  audio_stats: "音频统计",
  audio_output: "音频输出",
  av_sync: "音视频同步",
  video_gap: "帧间间隔",
  seek: "跳转",
  audio_resume: "音频恢复",
  decode_error: "解码错误",
  hw_decode_fallback: "硬解回退",
  decode_error_detail: "解码错误细节",
  stop: "停止",
};

/** Shown in the decode banner; snapshot no longer carries duplicate `hw_decode` rows. */
export interface DecodeBannerState {
  /** True when hardware decode is active, false for software. */
  isHardware: boolean;
  /** Raw mode from state: auto | on | off */
  mode: string;
  /** Short Chinese label for preferences column. */
  modeLabel: string;
  backend: string;
  error: string | null;
}

export interface DebugRow {
  key: string;
  label: string;
  value: string;
}

export interface DebugGroup {
  id: string;
  title: string;
  rows: DebugRow[];
}

export interface ProcessStage {
  id: string;
  label: string;
  status: "pending" | "active" | "completed" | "error";
  message: string;
  atMs: number | null;
}

export interface CurrentFrameSection {
  id: string;
  title: string;
  rows: DebugRow[];
}

const DEBUG_GROUP_ORDER = ["input", "stream", "decode", "video", "audio", "timing", "error", "other"] as const;

export function usePlayerDebugOverlay(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
  debugTimeline?: Ref<Array<{ stage: string; message: string; at_ms: number }>>,
  debugStageSnapshot?: Ref<Record<string, { message: string; at_ms: number }>>,
  latestTelemetry?: Ref<MediaTelemetryPayload | null>,
) {
  const decodeBanner = computed((): DecodeBannerState | null => {
    const state = playback.value;
    if (!state) {
      return null;
    }
    const mode = state.hw_decode_mode || "auto";
    return {
      isHardware: state.hw_decode_active,
      mode,
      modeLabel: formatHwModeLabel(mode),
      backend: state.hw_decode_backend || "—",
      error: state.hw_decode_error,
    };
  });

  const resourceSummary = computed(() => debugSnapshot.value.telemetry_resources || "");

  const debugRows = computed(() => {
    const snapshot = debugSnapshot.value;
    const rows: DebugRow[] = [];
    for (const key of PREFERRED_DEBUG_ORDER) {
      const value = snapshot[key];
      if (!value) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    for (const [key, value] of Object.entries(snapshot)) {
      if (key === "hw_decode" || key === "telemetry_resources" || !value) continue;
      if (PREFERRED_DEBUG_ORDER.includes(key as (typeof PREFERRED_DEBUG_ORDER)[number])) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    if (!rows.length) {
      return [{ key: "empty", label: "status", value: "等待解析信息..." }];
    }
    return rows;
  });

  const debugGroups = computed((): DebugGroup[] => {
    const bucketMap = new Map<string, DebugRow[]>();
    for (const id of DEBUG_GROUP_ORDER) {
      bucketMap.set(id, []);
    }
    for (const row of debugRows.value) {
      const groupId = detectDebugGroup(row.key);
      bucketMap.get(groupId)?.push(row);
    }
    return DEBUG_GROUP_ORDER.map((id) => ({
      id,
      title: formatGroupTitle(id),
      rows: bucketMap.get(id) ?? [],
    })).filter((group) => group.rows.length > 0);
  });

  const groupsById = computed(() => {
    const map = new Map<string, DebugGroup>();
    for (const group of debugGroups.value) {
      map.set(group.id, group);
    }
    return map;
  });

  const overviewGroups = computed(() =>
    ["video", "audio"]
      .map((id) => groupsById.value.get(id))
      .filter((group): group is DebugGroup => Boolean(group)),
  );

  const streamGroups = computed(() =>
    ["input", "stream", "decode"]
      .map((id) => groupsById.value.get(id))
      .filter((group): group is DebugGroup => Boolean(group)),
  );

  const timingGroups = computed(() =>
    ["timing", "error"]
      .map((id) => groupsById.value.get(id))
      .filter((group): group is DebugGroup => Boolean(group)),
  );

  const currentFrameSections = computed((): CurrentFrameSection[] => {
    const telemetry = latestTelemetry?.value;
    const snapshot = debugSnapshot.value;
    const timingRows: DebugRow[] = [];
    const outputRows: DebugRow[] = [];
    const decodeRows: DebugRow[] = [];

    const videoPts = telemetry?.current_video_pts_seconds;
    if (isFiniteNumber(videoPts)) {
      timingRows.push({
        key: "current_video_pts_seconds",
        label: "video pts",
        value: `${videoPts.toFixed(3)}s`,
      });
    }

    const clockSeconds = telemetry?.clock_seconds;
    if (isFiniteNumber(clockSeconds)) {
      timingRows.push({
        key: "clock_seconds",
        label: "play clock",
        value: `${clockSeconds.toFixed(3)}s`,
      });
    }

    const audioClock = telemetry?.current_audio_clock_seconds;
    if (isFiniteNumber(audioClock)) {
      timingRows.push({
        key: "current_audio_clock_seconds",
        label: "audio clock",
        value: `${audioClock.toFixed(3)}s`,
      });
    }

    const driftSeconds = telemetry?.audio_drift_seconds;
    if (isFiniteNumber(driftSeconds)) {
      timingRows.push({
        key: "audio_drift_seconds",
        label: "av drift",
        value: `${(driftSeconds * 1000).toFixed(2)}ms`,
      });
    }

    const ptsGap = telemetry?.video_pts_gap_seconds;
    if (isFiniteNumber(ptsGap)) {
      timingRows.push({
        key: "video_pts_gap_seconds",
        label: "frame gap",
        value: `${(ptsGap * 1000).toFixed(2)}ms`,
      });
    }

    const frameType = telemetry?.current_frame_type?.trim();
    if (frameType) {
      outputRows.push({
        key: "current_frame_type",
        label: "frame type",
        value: frameType,
      });
    }

    const width = telemetry?.current_frame_width;
    const height = telemetry?.current_frame_height;
    if (isFiniteNumber(width) && isFiniteNumber(height) && width > 0 && height > 0) {
      outputRows.push({
        key: "current_frame_size",
        label: "frame size",
        value: `${width}x${height}`,
      });
    }

    const renderFps = telemetry?.render_fps;
    if (isFiniteNumber(renderFps)) {
      outputRows.push({
        key: "render_fps",
        label: "render fps",
        value: `${renderFps.toFixed(2)}fps`,
      });
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
    }

    const playbackRate = telemetry?.playback_rate ?? playback.value?.playback_rate;
    if (isFiniteNumber(playbackRate)) {
      outputRows.push({
        key: "playback_rate",
        label: "rate",
        value: `${playbackRate.toFixed(2)}x`,
      });
    }

    const renderLag = telemetry?.render_present_lag_ms;
    if (isFiniteNumber(renderLag)) {
      decodeRows.push({
        key: "render_present_lag_ms",
        label: "present lag",
        value: `${renderLag.toFixed(2)}ms`,
      });
    }

    const decodeAvg = telemetry?.decode_avg_frame_cost_ms;
    if (isFiniteNumber(decodeAvg)) {
      decodeRows.push({
        key: "decode_avg_frame_cost_ms",
        label: "decode avg",
        value: `${decodeAvg.toFixed(2)}ms`,
      });
    }

    const decodeMax = telemetry?.decode_max_frame_cost_ms;
    if (isFiniteNumber(decodeMax)) {
      decodeRows.push({
        key: "decode_max_frame_cost_ms",
        label: "decode max",
        value: `${decodeMax.toFixed(2)}ms`,
      });
    }

    const decodeSamples = telemetry?.decode_samples;
    if (typeof decodeSamples === "number" && decodeSamples > 0) {
      decodeRows.push({
        key: "decode_samples",
        label: "window",
        value: `${decodeSamples} frames`,
      });
    }

    const integrity = snapshot.video_integrity;
    if (integrity) {
      decodeRows.push({
        key: "video_integrity",
        label: "integrity",
        value: integrity,
      });
    }

    return [
      { id: "timing", title: "帧时序", rows: timingRows },
      { id: "output", title: "输出状态", rows: outputRows },
      { id: "decode", title: "解码/呈现", rows: decodeRows },
    ].filter((section) => section.rows.length > 0);
  });

  const processStages = computed((): ProcessStage[] => {
    const latestByStage = new Map<string, { message: string; atMs: number }>();
    const stageSnapshot = debugStageSnapshot?.value ?? {};
    let hasError = false;
    for (const [stage, entry] of Object.entries(stageSnapshot)) {
      latestByStage.set(stage, { message: entry.message, atMs: entry.at_ms });
      if (stage.includes("error")) {
        hasError = true;
      }
    }
    if (!latestByStage.size) {
      const timeline = debugTimeline?.value ?? [];
      for (const item of timeline) {
        latestByStage.set(item.stage, { message: item.message, atMs: item.at_ms });
        if (item.stage.includes("error")) {
          hasError = true;
        }
      }
    }
    const stageDefs = [
      { id: "open", label: "打开源", aliases: ["open"] },
      { id: "stream_start", label: "启动流", aliases: ["stream_start"] },
      { id: "metadata_ready", label: "元数据", aliases: ["metadata_ready", "video_format", "video_stream"] },
      { id: "audio_pipeline_ready", label: "音频管线", aliases: ["audio", "audio_format", "audio_pipeline_ready"] },
      { id: "decoder_ready", label: "解码器", aliases: ["decoder_ready", "video_codec_profile"] },
      { id: "running", label: "进入播放", aliases: ["running"] },
      { id: "first_frame", label: "首帧输出", aliases: ["first_frame", "video_frame_format"] },
    ] as const;

    let activeAssigned = false;
    const hasFirstFrame = stageDefs
      .find((stage) => stage.id === "first_frame")
      ?.aliases.some((alias) => latestByStage.has(alias)) ?? false;

    const stages = stageDefs.map((stage) => {
      const matched = stage.aliases
        .map((alias) => latestByStage.get(alias))
        .filter((value): value is { message: string; atMs: number } => Boolean(value))
        .sort((a, b) => b.atMs - a.atMs)[0];
      if (matched) {
        return {
          id: stage.id,
          label: stage.label,
          status: hasError && stage.id === "first_frame" && !hasFirstFrame ? "error" : "completed",
          message: matched.message,
          atMs: matched.atMs,
        } satisfies ProcessStage;
      }
      if (!activeAssigned && !hasFirstFrame) {
        activeAssigned = true;
        return {
          id: stage.id,
          label: stage.label,
          status: hasError ? "error" : "active",
          message: hasError ? "流程中断或等待恢复" : "等待执行",
          atMs: null,
        } satisfies ProcessStage;
      }
      return {
        id: stage.id,
        label: stage.label,
        status: "pending",
        message: "等待执行",
        atMs: null,
      } satisfies ProcessStage;
    });
    const visibleStages = stages.filter((stage) => stage.status !== "pending");
    return visibleStages.length ? visibleStages : stages.slice(0, 1);
  });

  return {
    decodeBanner,
    resourceSummary,
    debugGroups,
    currentFrameSections,
    overviewGroups,
    streamGroups,
    timingGroups,
    processStages,
  };
}

function isFiniteNumber(value: number | null | undefined): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function formatDebugLabel(key: string): string {
  return DEBUG_LABELS[key] || key;
}

function formatHwModeLabel(mode: string): string {
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

function detectDebugGroup(key: string): string {
  if (key === "open" || key === "video_demux" || key === "video_gop") return "input";
  if (
    key === "metadata_ready"
    || key === "video_format"
    || key === "video_stream"
    || key === "audio"
    || key === "audio_format"
  ) {
    return "stream";
  }
  if (
    key === "decoder_ready"
    || key === "hw_decode_decision"
    || key === "audio_pipeline_ready"
    || key.startsWith("decode")
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
  if (key.endsWith("error")) return "error";
  if (key === "hw_decode_fallback") return "error";
  return "other";
}

function formatGroupTitle(groupId: string): string {
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
