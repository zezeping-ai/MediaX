<script setup lang="ts">
import { defineAsyncComponent, toRef } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { usePlayerDebugPanelViewModel } from "./usePlayerDebugPanelViewModel";

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

const {
  activeTab,
  decodeBanner,
  debugGroups,
  currentFrameSections,
  hardwareDecisionTimeline,
  liveBadgeClass,
  liveBadgeText,
  mediaInfoGroups,
  overviewSections,
  pipelineSections,
  streamSections,
  tabOptions,
  timingSections,
  processStages,
} = usePlayerDebugPanelViewModel({
  source: toRef(props, "source"),
  playback: toRef(props, "playback"),
  debugSnapshot: toRef(props, "debugSnapshot"),
  debugTimeline: toRef(props, "debugTimeline"),
  debugStageSnapshot: toRef(props, "debugStageSnapshot"),
  latestTelemetry: toRef(props, "latestTelemetry"),
  mediaInfoSnapshot: toRef(props, "mediaInfoSnapshot"),
});
</script>

<template>
  <div
    class="debug-overlay absolute left-4 top-4 z-5 flex h-[min(58vh,430px)] min-h-[250px] w-[min(620px,calc(100vw-32px))] min-w-[380px] max-h-[calc(100vh-24px)] max-w-[calc(100vw-24px)] resize flex-col gap-1.5 overflow-hidden rounded-xl border border-white/16 bg-[linear-gradient(180deg,rgba(11,16,23,0.86)_0%,rgba(9,13,20,0.78)_100%)] px-2 py-2 pl-2.5 font-mono text-[11px] leading-4 text-slate-100/95 shadow-[0_10px_30px_rgba(0,0,0,0.28)] backdrop-blur-[14px] max-[720px]:min-w-[320px]"
  >
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-1.5">
        <div class="font-bold tracking-[0.2px]">调试面板</div>
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
  color-scheme: dark;
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
