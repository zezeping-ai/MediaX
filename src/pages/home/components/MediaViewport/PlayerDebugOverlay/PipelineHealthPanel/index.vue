<script setup lang="ts">
import { usePipelineHealthLanes } from "./usePipelineHealthLanes";
import type { PipelineHealthPanelProps } from "./pipelineHealthPanel.types";

const props = defineProps<PipelineHealthPanelProps>();

const { lanes } = usePipelineHealthLanes(props);
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-1 text-slate-400/95">管线热度</div>
    <div v-if="!lanes.length" class="text-slate-100/70">等待管线负载采样...</div>
    <div v-else class="space-y-1">
      <div
        v-for="item in lanes"
        :key="item.id"
        class="grid grid-cols-[42px_1fr_90px] items-center gap-2"
      >
        <div class="text-[10px] text-slate-300/90">{{ item.label }}</div>
        <div class="relative h-8 overflow-hidden rounded-md border border-white/8 bg-slate-950/35">
          <div
            v-for="marker in item.markerPercents"
            :key="`${item.id}-${marker}`"
            class="absolute inset-y-0 w-px bg-white/10"
            :style="{ left: `${marker}%` }"
          />
          <div
            class="absolute bottom-0 left-0 rounded-md bg-gradient-to-r"
            :class="item.toneClass"
            :style="{ width: `${Math.max(6, item.percent)}%`, height: '10px' }"
          />
          <svg
            v-if="item.points"
            viewBox="0 0 96 18"
            preserveAspectRatio="none"
            class="absolute inset-x-1 top-1 h-4 w-[calc(100%-8px)]"
          >
            <polyline
              :points="item.points"
              fill="none"
              stroke="rgba(248,250,252,0.9)"
              stroke-width="1.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        </div>
        <div class="text-right text-[10px] text-slate-300/90">
          <div>{{ item.state }}</div>
          <div class="text-slate-500/90">{{ item.detail }}</div>
          <div class="text-slate-500/75">{{ item.trendLabel }} · {{ item.trendDelta }}</div>
        </div>
      </div>
    </div>
  </div>
</template>
