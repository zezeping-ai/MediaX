<script setup lang="ts">
import { computed, defineAsyncComponent, ref, toRef, watch } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { usePlayerDebugOverlay } from "../../../composables/usePlayerDebugOverlay";
import {
  STATIC_DEBUG_KEYS,
  formatMediaInfoLabel,
} from "./playerDebugOverlay.utils";

const ProcessTab = defineAsyncComponent({
  loader: () => import("./tabs/ProcessTab.vue"),
  delay: 80,
});

const OverviewTab = defineAsyncComponent({
  loader: () => import("./tabs/OverviewTab.vue"),
  delay: 80,
});

const PipelineTab = defineAsyncComponent({
  loader: () => import("./tabs/PipelineTab.vue"),
  delay: 80,
});

const CurrentFrameTab = defineAsyncComponent({
  loader: () => import("./tabs/CurrentFrameTab.vue"),
  delay: 80,
});

const SectionTab = defineAsyncComponent({
  loader: () => import("./tabs/SectionTab.vue"),
  delay: 80,
});
const props = defineProps<{
  source: string;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
  debugTimeline: Array<{ stage: string; message: string; at_ms: number }>;
  debugStageSnapshot: Record<string, { message: string; at_ms: number }>;
  latestTelemetry: MediaTelemetryPayload | null;
  telemetryHistory: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
  mediaInfoSnapshot: Record<string, string>;
}>();

const emit = defineEmits<{
  close: [];
}>();

const mediaInfoGroups = computed(() => {
  const baseRows: Array<{ key: string; label: string; value: string }> = [];
  const record = props.mediaInfoSnapshot || {};
  for (const [key, value] of Object.entries(record)) {
    if (!value) continue;
    baseRows.push({ key, label: formatMediaInfoLabel(key), value });
  }
  const videoRows: Array<{ key: string; label: string; value: string }> = [];
  const audioRows: Array<{ key: string; label: string; value: string }> = [];
  const videoFormat = props.debugSnapshot?.video_format;
  const videoCodecProfile = props.debugSnapshot?.video_codec_profile;
  const videoStream = props.debugSnapshot?.video_stream;
  const videoFrameFormat = props.debugSnapshot?.video_frame_format;
  const audioFormat = props.debugSnapshot?.audio_format;
  const audioStream = props.debugSnapshot?.audio;
  if (videoFormat) videoRows.push({ key: "video_format", label: formatMediaInfoLabel("video_format"), value: videoFormat });
  if (videoCodecProfile)
    videoRows.push({
      key: "video_codec_profile",
      label: formatMediaInfoLabel("video_codec_profile"),
      value: videoCodecProfile,
    });
  if (videoStream) videoRows.push({ key: "video_stream", label: formatMediaInfoLabel("video_stream"), value: videoStream });
  if (videoFrameFormat)
    videoRows.push({
      key: "video_frame_format",
      label: formatMediaInfoLabel("video_frame_format"),
      value: videoFrameFormat,
    });
  if (audioFormat) audioRows.push({ key: "audio_format", label: formatMediaInfoLabel("audio_format"), value: audioFormat });
  if (audioStream) audioRows.push({ key: "audio", label: formatMediaInfoLabel("audio"), value: audioStream });
  return [
    { id: "base", title: "基础", rows: baseRows },
    { id: "video", title: "视频", rows: videoRows },
    { id: "audio", title: "音频", rows: audioRows },
  ].filter((group) => group.rows.length > 0);
});

const realtimeDebugSnapshot = computed<Record<string, string>>(() => {
  const snapshot = props.debugSnapshot || {};
  const record: Record<string, string> = {};
  for (const [key, value] of Object.entries(snapshot)) {
    if (!value) continue;
    if (STATIC_DEBUG_KEYS.includes(key as (typeof STATIC_DEBUG_KEYS)[number])) continue;
    if (key === "telemetry_resources") continue;
    if (key === "telemetry_render") continue;
    record[key] = value;
  }
  return record;
});

const {
  decodeBanner,
  debugGroups,
  currentFrameSections,
  hardwareDecisionTimeline,
  overviewSections,
  pipelineSections,
  streamSections,
  timingSections,
  processStages,
} = usePlayerDebugOverlay(
  toRef(props, "playback"),
  realtimeDebugSnapshot,
  toRef(props, "debugTimeline"),
  toRef(props, "debugStageSnapshot"),
  toRef(props, "latestTelemetry"),
);
const activeTab = ref<"process" | "overview" | "pipeline" | "current-frame" | "stream" | "timing" | "runtime">("process");

const liveBadgeClass = computed(() => {
  if (!decodeBanner.value) return "border-blue-500/45 bg-blue-500/20 text-emerald-100";
  return decodeBanner.value.isHardware
    ? "border-emerald-400/55 bg-emerald-500/20 text-emerald-50"
    : "border-amber-300/55 bg-amber-500/20 text-orange-50";
});

const liveBadgeText = computed(() => {
  if (!decodeBanner.value) return "LIVE";
  return decodeBanner.value.isHardware ? "硬解" : "软解";
});

const tabOptions = [
  { label: "过程", value: "process" },
  { label: "概览", value: "overview" },
  { label: "管线", value: "pipeline" },
  { label: "当前帧", value: "current-frame" },
  { label: "流", value: "stream" },
  { label: "时序", value: "timing" },
  { label: "运行态", value: "runtime" },
] as const;

watch(
  () => props.source,
  () => {
    activeTab.value = "process";
  },
);
</script>

<template>
  <div
    class="debug-overlay absolute left-4 top-4 z-5 flex h-[min(58vh,430px)] min-h-[250px] w-[min(620px,calc(100vw-32px))] min-w-[380px] max-h-[calc(100vh-24px)] max-w-[calc(100vw-24px)] resize flex-col gap-1.5 overflow-hidden rounded-xl border border-white/16 bg-[linear-gradient(180deg,rgba(11,16,23,0.86)_0%,rgba(9,13,20,0.78)_100%)] px-2 py-2 pl-2.5 font-mono text-[11px] leading-4 text-slate-100/95 shadow-[0_10px_30px_rgba(0,0,0,0.28)] backdrop-blur-[14px] max-[720px]:min-w-[320px]"
  >
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-1.5">
        <div class="font-bold tracking-[0.2px]">解析 Debug</div>
        <span
          class="inline-flex h-4 items-center justify-center rounded-full border px-1.5 text-[10px]"
          :class="liveBadgeClass"
        >
          {{ liveBadgeText }}
        </span>
      </div>
      <a-button class="debug-close-btn" size="mini" type="text" @click="emit('close')">关闭</a-button>
    </div>

    <a-segmented
      v-model:value="activeTab"
      :options="tabOptions"
      size="small"
      class="debug-tab-nav"
    />

    <div class="debug-scroll-wrap flex min-h-0 flex-1 flex-col gap-1.5 overflow-auto pr-0.5">
      <template v-if="activeTab === 'process'">
        <ProcessTab :stages="processStages" :timeline="debugTimeline" />
      </template>

      <template v-else-if="activeTab === 'overview'">
        <OverviewTab
          :media-info-groups="mediaInfoGroups"
          :decode-banner="decodeBanner"
          :hardware-decision-timeline="hardwareDecisionTimeline"
          :overview-sections="overviewSections"
        />
      </template>

      <template v-else-if="activeTab === 'current-frame'">
        <CurrentFrameTab :sections="currentFrameSections" />
      </template>

      <template v-else-if="activeTab === 'pipeline'">
        <PipelineTab
          :telemetry="latestTelemetry"
          :history="telemetryHistory"
          :sections="pipelineSections"
        />
      </template>

      <template v-else-if="activeTab === 'stream'">
        <SectionTab
          title="输入源 / 流结构 / 解码链"
          :groups="streamSections"
          empty-text="等待输入源与解码链数据..."
        />
      </template>

      <template v-else-if="activeTab === 'timing'">
        <SectionTab
          title="同步质量 / 帧节奏 / 性能预算"
          :groups="timingSections"
          empty-text="等待时序与同步风险数据..."
        />
      </template>

      <template v-else>
        <SectionTab
          title="运行态全量视图"
          :groups="debugGroups"
          empty-text="等待运行态数据..."
        />
      </template>
    </div>
  </div>
</template>

<style scoped>
.debug-scroll-wrap {
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.42) transparent;
  color-scheme: dark;
}
.debug-scroll-wrap::-webkit-scrollbar {
  width: 6px;
  height: 6px;
  background: transparent;
}
.debug-scroll-wrap::-webkit-scrollbar-track {
  background: transparent;
}
.debug-scroll-wrap::-webkit-scrollbar-corner {
  background: transparent;
}
.debug-scroll-wrap::-webkit-scrollbar-thumb {
  border-radius: 999px;
  border: 1px solid transparent;
  background-clip: padding-box;
  background: rgba(148, 163, 184, 0.4);
  transition: background-color 180ms ease;
}
.debug-scroll-wrap:hover::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.65);
}
.debug-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.86);
  padding: 0 8px;
  height: 22px;
  min-width: 46px;
  line-height: 1;
  border-radius: 6px;
}
.debug-close-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.12);
}

.debug-tab-nav :deep(.arco-segmented) {
  background: rgba(255, 255, 255, 0.05);
}
</style>
