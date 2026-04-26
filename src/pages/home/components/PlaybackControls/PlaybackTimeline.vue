<script setup lang="ts">
import { computed } from "vue";
import type { PreviewFrame } from "@/modules/media-types";
import TimelineHoverPreview from "./TimelineHoverPreview.vue";

const props = defineProps<{
  currentTime: number;
  duration: number;
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
</template>
