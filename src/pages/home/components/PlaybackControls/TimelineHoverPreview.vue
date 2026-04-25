<script setup lang="ts">
import { onBeforeUnmount } from "vue";
import type { PreviewFrame } from "@/modules/media-types";
import { useTimelineHoverPreview } from "../../composables/useTimelineHoverPreview";
import { formatSeconds } from "./playbackControlsUtils";

type RequestPreviewFrame = (
  positionSeconds: number,
  maxWidth?: number,
  maxHeight?: number,
) => Promise<PreviewFrame | null>;

const props = defineProps<{
  durationSeconds: number;
  sourceKey: string;
  requestPreviewFrame?: RequestPreviewFrame;
}>();

const hoverPreview = useTimelineHoverPreview(
  () => props.durationSeconds,
  () => props.sourceKey,
  props.requestPreviewFrame,
);

// Vue templates don't auto-unwrap refs nested under objects.
// Expose them as top-level bindings to avoid "[object Object]" / NaN rendering.
const previewContainerRef = hoverPreview.previewContainerRef;
void previewContainerRef;
const canShowPreview = hoverPreview.canShowPreview;
const hoverLeft = hoverPreview.hoverLeft;
const hoverSeconds = hoverPreview.hoverSeconds;
const hoverImageSrc = hoverPreview.hoverImageSrc;
const hoverImageWidth = hoverPreview.hoverImageWidth;
const hoverImageHeight = hoverPreview.hoverImageHeight;
const onTimelineMouseMove = hoverPreview.onTimelineMouseMove;
const onTimelineMouseLeave = hoverPreview.onTimelineMouseLeave;

onBeforeUnmount(() => {
  hoverPreview.dispose();
});
</script>

<template>
  <div
    ref="previewContainerRef"
    class="relative"
    @mousemove="onTimelineMouseMove"
    @mouseleave="onTimelineMouseLeave"
  >
    <slot />

    <div
      v-if="canShowPreview"
      class="pointer-events-none absolute bottom-[calc(100%+10px)] z-10 flex -translate-x-1/2 flex-col items-center gap-1.5"
      :style="{ left: `${hoverLeft}px` }"
    >
      <img
        :src="hoverImageSrc"
        :width="hoverImageWidth"
        :height="hoverImageHeight"
        class="rounded-lg border border-white/15 bg-black/50 shadow-[0_14px_40px_rgba(0,0,0,0.55)]"
        alt="preview frame"
      />
      <span class="rounded-md border border-white/10 bg-black/55 px-2 py-0.5 text-[11px] leading-4 text-white/90">
        {{ formatSeconds(hoverSeconds) }}
      </span>
    </div>
  </div>
</template>

