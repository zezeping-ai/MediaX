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

    <TimelineHoverPreview
      :duration-seconds="previewDuration"
      :source-key="sourceKey"
      :request-preview-frame="requestPreviewFrame"
    >
      <a-slider
        class="w-full [&_.ant-slider]:m-0! [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.26)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-95 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:h-[3px] [&_.ant-slider-track]:bg-white/85"
        :value="currentTime"
        :max="sliderMax"
        :disabled="timelineDisabled"
        :title="timelineTitle"
        :tooltip-open="false"
        @update:value="$emit('preview', $event)"
        @change="$emit('commit', $event)"
      />
    </TimelineHoverPreview>
  </div>
</template>
