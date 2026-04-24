import { computed, type Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

const PREFERRED_DEBUG_ORDER = [
  "open",
  "hw_decode",
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
  open: "打开源",
  hw_decode: "解码模式",
  decoder_ready: "解码器",
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

export function usePlayerDebugOverlay(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
) {
  const hwDecodeLabel = computed(() => {
    const state = playback.value;
    if (!state) {
      return "";
    }
    const active = state.hw_decode_active ? "on" : "off";
    const backend = state.hw_decode_backend || "<none>";
    const mode = state.hw_decode_mode || "auto";
    const err = state.hw_decode_error ? ` | err=${state.hw_decode_error}` : "";
    return `hw_decode mode=${mode} active=${active} backend=${backend}${err}`;
  });

  const debugRows = computed(() => {
    const snapshot = debugSnapshot.value;
    const rows: Array<{ key: string; label: string; value: string }> = [];
    for (const key of PREFERRED_DEBUG_ORDER) {
      const value = snapshot[key];
      if (!value) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    for (const [key, value] of Object.entries(snapshot)) {
      if (!value || PREFERRED_DEBUG_ORDER.includes(key as (typeof PREFERRED_DEBUG_ORDER)[number])) continue;
      rows.push({ key, label: formatDebugLabel(key), value });
    }
    if (!rows.length) {
      return [{ key: "empty", label: "status", value: "等待解析信息..." }];
    }
    return rows;
  });

  return {
    hwDecodeLabel,
    debugRows,
  };
}

function formatDebugLabel(key: string): string {
  return DEBUG_LABELS[key] || key;
}
