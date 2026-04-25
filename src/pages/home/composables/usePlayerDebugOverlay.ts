import { computed, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

const PREFERRED_DEBUG_ORDER = [
  "open",
  "video_demux",
  "video_gop",
  "decoder_ready",
  "running",
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
] as const;

const DEBUG_LABELS: Record<string, string> = {
  open: "打开",
  stream_start: "流启动",
  decoder_ready: "解码器就绪",
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

const DEBUG_GROUP_ORDER = ["input", "decode", "video", "audio", "timing", "error", "other"] as const;

export function usePlayerDebugOverlay(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
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

  return {
    decodeBanner,
    resourceSummary,
    debugGroups,
  };
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
  if (key === "decoder_ready" || key.startsWith("decode")) return "decode";
  if (
    key === "running" ||
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
  return "other";
}

function formatGroupTitle(groupId: string): string {
  switch (groupId) {
    case "input":
      return "输入/流";
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
