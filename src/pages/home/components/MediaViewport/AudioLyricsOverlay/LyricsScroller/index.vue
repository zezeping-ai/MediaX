<script setup lang="ts">
import type { MediaLyricLine } from "@/modules/media-types";
import { useLyricsScrollerProps } from "./useLyricsScroller";

const props = defineProps<{
  lines: MediaLyricLine[];
  activeIndex: number;
  playbackPositionSeconds: number;
  fetching: boolean;
  dense: boolean;
  transparentOverlay: boolean;
  isDark?: boolean;
  dragEnabled?: boolean;
  baseOffsetSeconds?: number;
}>();

const emit = defineEmits<{
  "drag-preview": [deltaSeconds: number];
  "offset-commit": [nextOffsetSeconds: number];
  "dragging-change": [dragging: boolean];
}>();

const {
  blockWindowDrag,
  centerRailClass,
  contentRef,
  contentStyle,
  dragging,
  edgeFadeBottomClass,
  edgeFadeTopClass,
  emptyStateClass,
  handlePointerCancel,
  handlePointerDown,
  handlePointerMove,
  handlePointerUp,
  highlightedIndex,
  lineSlotClass,
  lineTextClass,
  listPaddingY,
  setLineRef,
  viewportClass,
  viewportRef,
} = useLyricsScrollerProps(props, emit);
</script>

<template>
  <div class="pointer-events-auto flex min-h-0 flex-1 flex-col">
    <div
      v-if="fetching && lines.length === 0"
      class="flex min-h-[10rem] flex-1 items-center justify-center text-sm tracking-wide"
      :class="emptyStateClass"
    >
      正在查询歌词…
    </div>

    <div
      v-else-if="lines.length === 0"
      class="flex min-h-[10rem] flex-1 items-center justify-center text-sm tracking-wide"
      :class="emptyStateClass"
    >
      未找到歌词
    </div>

    <div
      v-else
      ref="viewportRef"
      :class="viewportClass"
      data-no-window-drag="true"
      @mousedown.stop="blockWindowDrag"
      @pointerdown="(event) => handlePointerDown(event, viewportRef)"
      @pointermove="handlePointerMove"
      @pointerup="handlePointerUp"
      @pointercancel="handlePointerCancel"
    >
      <div
        v-if="dragEnabled"
        class="pointer-events-none absolute inset-x-3 top-1/2 z-30 h-8 -translate-y-1/2 rounded-md border"
        :class="centerRailClass"
      />

      <div
        class="pointer-events-none absolute inset-x-0 top-0 z-20 h-10 bg-linear-to-b to-transparent"
        :class="edgeFadeTopClass"
      />
      <div
        class="pointer-events-none absolute inset-x-0 bottom-0 z-20 h-10 bg-linear-to-t to-transparent"
        :class="edgeFadeBottomClass"
      />

      <div ref="contentRef" class="relative z-10 will-change-transform" :style="contentStyle">
        <div
          class="px-1"
          :style="{ paddingTop: `${listPaddingY}px`, paddingBottom: `${listPaddingY}px` }"
        >
          <p
            v-for="(line, index) in lines"
            :key="`${line.time_seconds}-${index}`"
            :ref="(element) => setLineRef(index, element)"
            class="flex items-center justify-center break-words text-center tracking-wide transition-opacity duration-300"
            :class="[
              lineSlotClass,
              index === highlightedIndex ? 'opacity-100' : 'opacity-70',
              dragging ? 'duration-0!' : '',
            ]"
            :aria-current="index === highlightedIndex ? 'true' : undefined"
          >
            <span
              class="inline-block transition-[color,background-color,box-shadow,text-shadow] duration-300"
              :class="lineTextClass(index === highlightedIndex)"
            >
              {{ line.text }}
            </span>
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
