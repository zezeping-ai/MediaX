<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import type { PreviewFrame } from "@/modules/media-types";
import { formatSeconds } from "./playbackControlsUtils";

const TimelineHoverPreview = defineAsyncComponent({
  loader: () => import("./TimelineHoverPreview.vue"),
  delay: 120,
});

const props = defineProps<{
  currentTime: number;
  bufferedTime: number;
  duration: number;
  decodeBadgeClass: string;
  decodeBadgeLabel: string;
  decodeBadgeTitle: string;
  sliderMax: number;
  timelineDisabled: boolean;
  timelineTitle: string;
  sourceKey: string;
  requestPreviewFrame?: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number
  ) => Promise<PreviewFrame | null>;
}>();

defineEmits<{
  preview: [number | [number, number]];
  commit: [number | [number, number]];
}>();

const previewDuration = computed(() => Math.max(props.duration, props.sliderMax));
const playedPercent = computed(() => {
  const max = Math.max(props.sliderMax, 1);
  const current = Math.max(0, Math.min(props.currentTime, max));
  return (current / max) * 100;
});
const bufferedPercent = computed(() => {
  const max = Math.max(props.sliderMax, 1);
  const buffered = Math.max(props.currentTime, Math.min(props.bufferedTime, max));
  return (buffered / max) * 100;
});

const timelineRailInsetPx = 5;
</script>

<template>
  <div class="space-y-1.5">
    <div class="flex items-center justify-between gap-3">
      <span
        class="inline-flex h-6 items-center rounded-md border px-2 text-[10px] font-semibold tracking-[0.1px] leading-none transition-colors duration-150"
        :class="[decodeBadgeClass, 'justify-center whitespace-nowrap']"
        :title="decodeBadgeTitle"
      >
        {{ decodeBadgeLabel }}
      </span>

      <div
        class="flex items-baseline gap-1.5 text-[11px] text-white/70 [font-variant-numeric:tabular-nums]"
      >
        <span class="text-white/85">{{ formatSeconds(currentTime) }}</span>
        <span class="text-white/35">/</span>
        <span class="text-white/60">{{ formatSeconds(duration) }}</span>
      </div>
    </div>

    <div
      class="relative"
      :style="{
        paddingInline: `${timelineRailInsetPx}px`,
      }"
    >
      <div
        class="pointer-events-none absolute top-1/2 z-0 h-[3px] -translate-y-1/2 overflow-hidden rounded-full bg-white/12"
        :style="{
          left: `${timelineRailInsetPx}px`,
          right: `${timelineRailInsetPx}px`,
        }"
      >
        <div
          class="absolute inset-y-0 left-0 rounded-full bg-white/25 transition-[width] duration-150"
          :style="{ width: `${bufferedPercent}%` }"
        />
        <div
          class="absolute inset-y-0 left-0 rounded-full bg-white/85 transition-[width] duration-100"
          :style="{ width: `${playedPercent}%` }"
        />
      </div>

      <TimelineHoverPreview
        :duration-seconds="previewDuration"
        :source-key="sourceKey"
        :request-preview-frame="requestPreviewFrame"
      >
        <a-slider
          class="relative z-10 !m-0 w-full [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.26)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-95 [&_.ant-slider-rail]:bg-transparent [&_.ant-slider-track]:bg-transparent"
          :value="currentTime"
          :max="sliderMax"
          :included="false"
          :disabled="timelineDisabled"
          :title="timelineTitle"
          :tooltip-open="false"
          @update:value="$emit('preview', $event)"
          @change="$emit('commit', $event)"
        />
      </TimelineHoverPreview>
    </div>
  </div>
</template>
