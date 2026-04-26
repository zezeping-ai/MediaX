import { computed, ref, watch, type Ref } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { usePlayerDebugOverlay } from "../../../composables/usePlayerDebugOverlay";
import { STATIC_DEBUG_KEYS, formatMediaInfoLabel } from "./playerDebugOverlay.utils";

type DebugTimelineEntry = { stage: string; message: string; at_ms: number };
type DebugStageSnapshotEntry = { message: string; at_ms: number };

type UsePlayerDebugPanelViewModelOptions = {
  source: Ref<string>;
  playback: Ref<PlaybackState | null>;
  debugSnapshot: Ref<Record<string, string>>;
  debugTimeline: Ref<DebugTimelineEntry[]>;
  debugStageSnapshot: Ref<Record<string, DebugStageSnapshotEntry>>;
  latestTelemetry: Ref<MediaTelemetryPayload | null>;
  mediaInfoSnapshot: Ref<Record<string, string>>;
};

export function usePlayerDebugPanelViewModel(options: UsePlayerDebugPanelViewModelOptions) {
  const mediaInfoGroups = computed(() => {
    const baseRows: Array<{ key: string; label: string; value: string }> = [];
    const record = options.mediaInfoSnapshot.value || {};
    for (const [key, value] of Object.entries(record)) {
      if (!value) continue;
      baseRows.push({ key, label: formatMediaInfoLabel(key), value });
    }
    const videoRows: Array<{ key: string; label: string; value: string }> = [];
    const audioRows: Array<{ key: string; label: string; value: string }> = [];
    const snapshot = options.debugSnapshot.value;
    if (snapshot.video_format) videoRows.push({ key: "video_format", label: formatMediaInfoLabel("video_format"), value: snapshot.video_format });
    if (snapshot.video_codec_profile) videoRows.push({ key: "video_codec_profile", label: formatMediaInfoLabel("video_codec_profile"), value: snapshot.video_codec_profile });
    if (snapshot.video_stream) videoRows.push({ key: "video_stream", label: formatMediaInfoLabel("video_stream"), value: snapshot.video_stream });
    if (snapshot.video_frame_format) videoRows.push({ key: "video_frame_format", label: formatMediaInfoLabel("video_frame_format"), value: snapshot.video_frame_format });
    if (snapshot.audio_format) audioRows.push({ key: "audio_format", label: formatMediaInfoLabel("audio_format"), value: snapshot.audio_format });
    if (snapshot.audio) audioRows.push({ key: "audio", label: formatMediaInfoLabel("audio"), value: snapshot.audio });
    return [
      { id: "base", title: "基础", rows: baseRows },
      { id: "video", title: "视频", rows: videoRows },
      { id: "audio", title: "音频", rows: audioRows },
    ].filter((group) => group.rows.length > 0);
  });

  const realtimeDebugSnapshot = computed<Record<string, string>>(() => {
    const record: Record<string, string> = {};
    for (const [key, value] of Object.entries(options.debugSnapshot.value || {})) {
      if (!value) continue;
      if (STATIC_DEBUG_KEYS.includes(key as (typeof STATIC_DEBUG_KEYS)[number])) continue;
      if (key === "telemetry_resources" || key === "telemetry_render") continue;
      record[key] = value;
    }
    return record;
  });

  const overlayState = usePlayerDebugOverlay(
    options.playback,
    realtimeDebugSnapshot,
    options.debugTimeline,
    options.debugStageSnapshot,
    options.latestTelemetry,
  );

  const activeTab = ref<"process" | "overview" | "pipeline" | "current-frame" | "stream" | "timing" | "runtime">("process");
  const tabOptions = [
    { label: "过程", value: "process" },
    { label: "概览", value: "overview" },
    { label: "管线", value: "pipeline" },
    { label: "当前帧", value: "current-frame" },
    { label: "流", value: "stream" },
    { label: "时序", value: "timing" },
    { label: "运行态", value: "runtime" },
  ] as const;

  const liveBadgeClass = computed(() => {
    if (!overlayState.decodeBanner.value) return "border-blue-500/45 bg-blue-500/20 text-emerald-100";
    return overlayState.decodeBanner.value.isHardware
      ? "border-emerald-400/55 bg-emerald-500/20 text-emerald-50"
      : "border-amber-300/55 bg-amber-500/20 text-orange-50";
  });

  const liveBadgeText = computed(() => {
    if (!overlayState.decodeBanner.value) return "LIVE";
    return overlayState.decodeBanner.value.isHardware ? "硬解" : "软解";
  });

  watch(options.source, () => {
    activeTab.value = "process";
  });

  return {
    ...overlayState,
    activeTab,
    liveBadgeClass,
    liveBadgeText,
    mediaInfoGroups,
    tabOptions,
  };
}
