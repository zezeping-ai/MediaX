import { computed, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

const PREFERRED_DEBUG_ORDER = [
  "open",
  "decoder_ready",
  "video_stream",
  "audio",
  "running",
  "video_pipeline",
  "video_integrity",
  "telemetry",
  "video_fps",
  "audio_stats",
  "video_gap",
  "seek",
  "audio_resume",
  "decode_error",
] as const;

const DEBUG_LABELS: Record<string, string> = {
  open: "打开",
  stream_start: "流启动",
  decoder_ready: "解码器就绪",
  color_profile: "色彩配置",
  color_profill: "色彩配置",
  video_stream: "视频流",
  audio: "音频流",
  running: "运行状态",
  video_pipeline: "视频管线",
  video_integrity: "完整性",
  telemetry: "时序指标",
  video_fps: "视频帧率",
  audio_stats: "音频统计",
  video_gap: "帧间间隔",
  seek: "跳转",
  audio_resume: "音频恢复",
  decode_error: "解码错误",
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

const DEBUG_GROUP_ORDER = ["open", "decode", "stream", "timing", "error", "other"] as const;

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

  const debugRows = computed(() => {
    const snapshot = debugSnapshot.value;
    const rows: DebugRow[] = [];
    for (const key of PREFERRED_DEBUG_ORDER) {
      const value = snapshot[key];
      if (!value) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    for (const [key, value] of Object.entries(snapshot)) {
      if (key === "hw_decode" || !value) continue;
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
  if (key === "open") return "open";
  if (key === "decoder_ready" || key.startsWith("decode")) return "decode";
  if (key === "video_stream" || key === "audio" || key === "audio_stats") return "stream";
  if (key === "running" || key.startsWith("video_") || key === "telemetry" || key === "seek" || key === "audio_resume") {
    return "timing";
  }
  if (key.endsWith("error")) return "error";
  return "other";
}

function formatGroupTitle(groupId: string): string {
  switch (groupId) {
    case "open":
      return "打开";
    case "decode":
      return "解码";
    case "stream":
      return "流信息";
    case "timing":
      return "播放/时序";
    case "error":
      return "异常";
    default:
      return "其他";
  }
}
