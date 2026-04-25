<script setup lang="ts">
import { computed, onBeforeUnmount } from "vue";
import { Icon } from "@iconify/vue";
import type { PlaybackState, PreviewFrame } from "@/modules/media-types";
import { useTimelineHoverPreview } from "../composables/useTimelineHoverPreview";
import { usePlaybackTimelineState } from "../composables/usePlaybackTimelineState";
import {
  CIRCLE_BTN_BASE,
  CIRCLE_BTN_GHOST,
  CIRCLE_BTN_PRIMARY,
  PILL_BASE,
  SPEED_OPTIONS,
  TINY_PILL_BTN,
} from "./playbackControls.constants";
import { formatSeconds } from "./playbackControls.utils";

const props = defineProps<{
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  volume: number;
  muted: boolean;
  locked: boolean;
  requestPreviewFrame?: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number
  ) => Promise<PreviewFrame | null>;
}>();

const emit = defineEmits<{
  play: [];
  pause: [number];
  stop: [];
  seek: [number];
  "seek-preview": [number];
  "change-rate": [number];
  "change-volume": [number];
  "toggle-mute": [];
  "toggle-lock": [];
}>();

function normalizeSliderValue(value: number | [number, number]) {
  return Array.isArray(value) ? Number(value[0]) : Number(value);
}

const { currentTime, commitSeek, previewSeekWhilePaused, cancelPreviewSeek } = usePlaybackTimelineState({
  playback: () => props.playback,
  onSeek: (seconds) => emit("seek", seconds),
  onSeekPreview: (seconds) => emit("seek-preview", seconds),
});
const duration = computed(() => props.playback?.duration_seconds ?? 0);
const sliderMax = computed(() => Math.max(duration.value, currentTime.value, 1));
const isPlaying = computed(() => props.playback?.status === "playing");
const volumeIcon = computed(() => {
  if (props.muted || props.volume <= 0) {
    return "lucide:volume-x";
  }
  if (props.volume < 0.5) {
    return "lucide:volume-1";
  }
  return "lucide:volume-2";
});
const speedLabel = computed(() => `${props.playbackRate}x`);

// 线性小图标：比 duotone 更轻，与音量区图标体量接近
const lockIcon = computed(() => (props.locked ? "lucide:lock" : "lucide:lock-open"));


function emitPause() {
  emit("pause", currentTime.value);
}

function handleSpeedMenuClick({ key }: { key: string | number }) {
  emit("change-rate", Number(key));
}

function handleProgressPreviewUpdate(value: number | [number, number]) {
  previewSeekWhilePaused(normalizeSliderValue(value));
}

function handleProgressCommit(value: number | [number, number]) {
  commitSeek(normalizeSliderValue(value));
}

function handleVolumeChange(value: number | [number, number]) {
  emit("change-volume", normalizeSliderValue(value));
}

const {
  previewContainerRef,
  hoverLeft,
  hoverSeconds,
  hoverImageSrc,
  hoverImageWidth,
  hoverImageHeight,
  canShowPreview,
  onTimelineMouseMove,
  onTimelineMouseLeave,
  dispose: disposeHoverPreview,
} = useTimelineHoverPreview(
  () => Math.max(duration.value, sliderMax.value),
  () => props.playback?.current_path ?? "",
  props.requestPreviewFrame
);

onBeforeUnmount(() => {
  cancelPreviewSeek();
  disposeHoverPreview();
});
</script>

<template>
  <section
    class="w-full overflow-visible rounded-t-2xl rounded-b-none border border-white/10 bg-[linear-gradient(180deg,rgba(0,0,0,0.25)_0%,rgba(0,0,0,0.35)_100%)] shadow-[0_18px_60px_rgba(0,0,0,0.55)] backdrop-blur-2xl"
  >
    <div class="px-3.5 pb-2 pt-2.5">
      <div
        ref="previewContainerRef"
        class="relative"
        @mousemove="onTimelineMouseMove"
        @mouseleave="onTimelineMouseLeave"
      >
        <a-slider
          class="w-full [&_.ant-slider]:m-0! [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:bg-white/85 [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.26)] [&_.ant-slider-handle]:opacity-95 [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-track]:h-[3px]"
          :value="currentTime"
          :max="sliderMax"
          :disabled="disabled"
          title="拖动调整播放进度"
          :tooltip-open="false"
          @update:value="handleProgressPreviewUpdate"
          @change="handleProgressCommit"
        />

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
          <span
            class="rounded-md border border-white/10 bg-black/55 px-2 py-0.5 text-[11px] leading-4 text-white/90"
          >
            {{ formatSeconds(hoverSeconds) }}
          </span>
        </div>
      </div>

      <div class="mt-1 grid grid-cols-[40px_minmax(0,1fr)_40px] items-center gap-2 max-[720px]:grid-cols-[34px_minmax(0,1fr)_34px]">
        <div aria-hidden="true" />
        <div class="flex justify-center">
          <div :class="[PILL_BASE, 'max-w-full gap-2.5 px-3']">
          <div
            class="flex items-baseline gap-1.5 text-[11px] text-white/70 [font-variant-numeric:tabular-nums]"
          >
            <span class="text-white/85">{{ formatSeconds(currentTime) }}</span>
            <span class="text-white/35">/</span>
            <span class="text-white/60">{{ formatSeconds(duration) }}</span>
          </div>

          <span class="h-5 w-px bg-white/10" aria-hidden="true" />

          <a-button
            size="small"
            shape="circle"
            :disabled="disabled"
            title="停止播放"
            :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST]"
            @click="emit('stop')"
          >
            <Icon
              icon="ph:stop-fill"
              width="16"
              height="16"
              class="block shrink-0"
              aria-hidden="true"
            />
          </a-button>

          <a-button
            size="small"
            shape="circle"
            :disabled="disabled"
            :title="isPlaying ? '暂停播放' : '开始播放'"
            :class="[CIRCLE_BTN_BASE, 'h-11 min-h-11 w-11 min-w-11', CIRCLE_BTN_PRIMARY]"
            @click="isPlaying ? emitPause() : emit('play')"
          >
            <Icon
              :icon="isPlaying ? 'ph:pause-fill' : 'ph:play-fill'"
              width="20"
              height="20"
              class="block shrink-0"
              aria-hidden="true"
            />
          </a-button>

          <span class="h-5 w-px bg-white/10" aria-hidden="true" />

          <a-dropdown :trigger="['click']" placement="top">
            <a-button
              size="small"
              :class="TINY_PILL_BTN"
              :disabled="disabled"
              title="调整播放倍速"
            >
              <span class="tabular-nums">{{ speedLabel }}</span>
              <Icon
                icon="mdi:chevron-up"
                width="14"
                height="14"
                class="shrink-0 opacity-75"
                aria-hidden="true"
              />
            </a-button>
            <template #overlay>
              <a-menu
                :selected-keys="[String(playbackRate)]"
                @click="handleSpeedMenuClick"
              >
                <a-menu-item v-for="value in SPEED_OPTIONS" :key="String(value)">
                  {{ value }}x
                </a-menu-item>
              </a-menu>
            </template>
          </a-dropdown>

          <span class="h-5 w-px bg-white/10" aria-hidden="true" />

          <a-button
            size="small"
            shape="circle"
            :disabled="disabled"
            :title="muted || volume <= 0 ? '取消静音' : '静音'"
            :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST]"
            @click="emit('toggle-mute')"
          >
            <Icon
              :icon="volumeIcon"
              width="18"
              height="18"
              class="block shrink-0"
              aria-hidden="true"
            />
          </a-button>

          <div class="ml-1 mr-2 w-[118px] max-[720px]:hidden">
            <a-slider
              class="w-full [&_.ant-slider]:m-0! [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:bg-white/70 [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.20)] [&_.ant-slider-handle]:opacity-90 [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-track]:h-[3px]"
              :value="muted ? 0 : volume"
              :min="0"
              :max="1"
              :step="0.01"
              :tooltip-open="false"
              :disabled="disabled"
              title="调整音量"
              @change="handleVolumeChange"
            />
          </div>

          </div>
        </div>

        <a-button
          type="text"
          size="small"
          shape="circle"
          class="justify-self-end max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
          :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST, locked ? 'bg-white/15 text-white' : '']"
          :title="locked ? '取消锁定控制器自动隐藏' : '锁定控制器常驻显示'"
          @click="emit('toggle-lock')"
        >
          <Icon
            :icon="lockIcon"
            width="15"
            height="15"
            class="block shrink-0"
            aria-hidden="true"
          />
        </a-button>
      </div>
    </div>
  </section>
</template>
