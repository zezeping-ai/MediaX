<script setup lang="ts">
import { computed } from "vue";
import { formatPercent } from "@/pages/home/components/PlaybackControls/PlaybackCenterControls/channelTrimDisplay";
import { usePipelineHealthLanes } from "./usePipelineHealthLanes";
import type { PipelineHealthPanelProps } from "./pipelineHealthPanel.types";

const props = defineProps<PipelineHealthPanelProps>();

const { lanes } = usePipelineHealthLanes(props);

const toneDotClassMap = {
  unknown: "bg-slate-400/55",
  headroom: "bg-sky-400/85",
  healthy: "bg-emerald-400/85",
  tight: "bg-amber-400/85",
  risk: "bg-rose-400/85",
} as const;

const toneTextClassMap = {
  unknown: "text-slate-300/78",
  headroom: "text-sky-200/88",
  healthy: "text-emerald-200/88",
  tight: "text-amber-200/88",
  risk: "text-rose-200/88",
} as const;

const laneSummaries = computed(() =>
  lanes.value.map((lane) => ({
    ...lane,
    percentLabel: formatPercent(lane.percent / 100),
    dotClass: toneDotClassMap[lane.tone],
    textClass: toneTextClassMap[lane.tone],
  })),
);
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-1 text-[11px] text-slate-400/95">管线热度</div>
    <div v-if="!lanes.length" class="text-slate-100/70">等待管线负载采样...</div>
    <div v-else class="grid grid-cols-2 gap-1.5 max-[720px]:grid-cols-1">
      <div
        v-for="item in laneSummaries"
        :key="item.id"
        class="rounded-md border border-white/7 bg-black/12 px-2 py-1.5"
      >
        <div class="mb-0.5 flex items-center justify-between text-[10px]">
          <div class="flex items-center gap-1.5 text-slate-200/92">
            <span class="h-1.5 w-1.5 rounded-full" :class="item.dotClass" />
            <span>{{ item.label }}</span>
          </div>
          <span class="tabular-nums text-slate-400/88">{{ item.percentLabel }}</span>
        </div>
        <div class="text-[10px]" :class="item.textClass">{{ item.state }}</div>
        <div class="text-[10px] text-slate-500/90">{{ item.detail }}</div>
        <div class="text-[10px] text-slate-500/72">{{ item.trendLabel }} · {{ item.trendDelta }}</div>
      </div>
    </div>
  </div>
</template>
