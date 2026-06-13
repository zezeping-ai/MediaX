<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import type { PreviewFrame } from "@/modules/media-types";
import ResumePlaybackPrompt from "./ResumePlaybackPrompt.vue";
import { formatSeconds } from "./playbackControlsUtils";
import { usePlayerChromeTheme } from "@/pages/home/composables/usePlayerChromeTheme";
import type { TimelineEmitContract, TimelineViewProps } from "./bindings.contract";

const TimelineHoverPreview = defineAsyncComponent({
  loader: () => import("./TimelineHoverPreview.vue"),
  delay: 120,
});

const props = defineProps<TimelineViewProps & {
  // keep local generic visible for template autocomplete
  requestPreviewFrame?: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number
  ) => Promise<PreviewFrame | null>;
}>();

defineEmits<TimelineEmitContract>();

const {
  isDark,
  timelineBuffered,
  timelinePlayed,
  timelineRail,
  timelineSlider,
  timelineTime,
  timelineTimeCurrent,
  timelineTimeMuted,
} = usePlayerChromeTheme();

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
      <ResumePlaybackPrompt
        v-if="resumePromptPositionSeconds != null"
        :position-seconds="resumePromptPositionSeconds"
        @accept="$emit('resume-prompt-accept')"
        @dismiss="$emit('resume-prompt-dismiss')"
      />
      <div
        class="ml-auto flex shrink-0 items-baseline gap-1.5 text-[11px] [font-variant-numeric:tabular-nums]"
        :class="timelineTime"
      >
        <span :class="timelineTimeCurrent">{{ formatSeconds(currentTime) }}</span>
        <span :class="isDark ? 'text-white/35' : 'text-slate-400'">/</span>
        <span :class="timelineTimeMuted">{{ formatSeconds(duration) }}</span>
      </div>
    </div>

    <div
      class="relative"
      :style="{
        paddingInline: `${timelineRailInsetPx}px`,
      }"
    >
      <div
        class="pointer-events-none absolute top-1/2 z-0 h-[3px] -translate-y-1/2 overflow-hidden rounded-full"
        :class="timelineRail"
        :style="{
          left: `${timelineRailInsetPx}px`,
          right: `${timelineRailInsetPx}px`,
        }"
      >
        <div
          class="absolute inset-y-0 left-0 rounded-full transition-[width] duration-150"
          :class="timelineBuffered"
          :style="{ width: `${bufferedPercent}%` }"
        />
        <div
          class="absolute inset-y-0 left-0 rounded-full transition-[width] duration-100"
          :class="timelinePlayed"
          :style="{ width: `${playedPercent}%` }"
        />
      </div>

      <TimelineHoverPreview
        :duration-seconds="previewDuration"
        :source-key="sourceKey"
        :request-preview-frame="requestPreviewFrame"
      >
        <a-slider
          class="relative z-10 m-0! w-full [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-95 [&_.ant-slider-rail]:bg-transparent [&_.ant-slider-track]:bg-transparent"
          :class="timelineSlider"
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
