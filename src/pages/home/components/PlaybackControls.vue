<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Icon } from "@iconify/vue";
import { throttle } from "lodash-es";
import type { PlaybackState, PreviewFrame } from "@/modules/media-types";
import { useTimelineHoverPreview } from "../composables/useTimelineHoverPreview";

const props = defineProps<{
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  volume: number;
  muted: boolean;
  locked: boolean;
  requestPreviewFrame?: (positionSeconds: number, maxWidth?: number, maxHeight?: number) => Promise<PreviewFrame | null>;
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

const nowTick = ref(Date.now());
const anchorPosition = ref(0);
const anchorAtMs = ref(Date.now());
let tickTimer: number | null = null;
const PREVIEW_SEEK_INTERVAL_MS = 100;

const currentTime = computed(() => {
  const playback = props.playback;
  if (!playback) {
    return 0;
  }
  if (playback.status !== "playing") {
    return anchorPosition.value;
  }
  const rate = playback.playback_rate > 0 ? playback.playback_rate : 1;
  const elapsedSeconds = Math.max(0, nowTick.value - anchorAtMs.value) / 1000;
  const progressed = anchorPosition.value + elapsedSeconds * rate;
  const maxDuration =
    playback.duration_seconds > 0 ? playback.duration_seconds : Number.POSITIVE_INFINITY;
  return Math.min(progressed, maxDuration);
});
const duration = computed(() => props.playback?.duration_seconds ?? 0);
const sliderMax = computed(() => Math.max(duration.value, currentTime.value, 1));
const isPlaying = computed(() => props.playback?.status === "playing");
const volumeIcon = computed(() => {
  if (props.muted || props.volume <= 0) {
    return "solar:volume-cross-bold-duotone";
  }
  if (props.volume < 0.5) {
    return "solar:volume-small-bold-duotone";
  }
  return "solar:volume-loud-bold-duotone";
});
const speedOptions = [0.25, 0.5, 0.75, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];

function formatSeconds(value: number) {
  const safeValue = Math.max(0, Math.floor(value || 0));
  const minutes = Math.floor(safeValue / 60);
  const seconds = safeValue % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}

function emitSeek(nextSeconds: number) {
  const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
  // Final seek on slider release should win. Cancel any trailing paused-preview
  // seek that may fire later from throttle, otherwise final frame commit can be
  // intermittently overridden/cancelled.
  emitPausedSeekPreview.cancel();
  anchorPosition.value = normalized;
  anchorAtMs.value = Date.now();
  nowTick.value = anchorAtMs.value;
  emit("seek", normalized);
}

function previewSeekWhilePaused(nextSeconds: number) {
  const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
  anchorPosition.value = normalized;
  anchorAtMs.value = Date.now();
  nowTick.value = anchorAtMs.value;
  if (props.playback?.status !== "paused") {
    emitPausedSeekPreview.cancel();
    return;
  }
  emitPausedSeekPreview(normalized);
}

function emitPause() {
  emit("pause", currentTime.value);
}

// Keep scrubbing responsive while avoiding frequent backend seek preview calls.
const emitPausedSeekPreview = throttle(
  (seconds: number) => emit("seek-preview", seconds),
  PREVIEW_SEEK_INTERVAL_MS,
  { leading: true, trailing: true },
);

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
  props.requestPreviewFrame,
);

function clearTickTimer() {
  if (tickTimer !== null) {
    window.clearInterval(tickTimer);
    tickTimer = null;
  }
}

function ensureTickTimer() {
  if (tickTimer !== null) {
    return;
  }
  tickTimer = window.setInterval(() => {
    nowTick.value = Date.now();
  }, 200);
}

watch(
  () => props.playback,
  (playback) => {
    if (!playback) {
      anchorPosition.value = 0;
      anchorAtMs.value = Date.now();
      clearTickTimer();
      return;
    }

    const backendPos = playback.position_seconds ?? 0;
    const drift = backendPos - anchorPosition.value;
    const backendJumped = Math.abs(drift) >= 0.5;
    const backendAdvanced = drift > 0.05;
    if (backendJumped || backendAdvanced || playback.status !== "playing") {
      anchorPosition.value = backendPos;
      anchorAtMs.value = Date.now();
    }

    if (playback.status === "playing") {
      ensureTickTimer();
      emitPausedSeekPreview.cancel();
    } else {
      clearTickTimer();
    }
  },
  { immediate: true, deep: true },
);

onBeforeUnmount(() => {
  clearTickTimer();
  emitPausedSeekPreview.cancel();
  disposeHoverPreview();
});

</script>

<template>
  <section class="playback-controls">
    <div class="top-row">
      <a-button
        class="lock-btn"
        shape="circle"
        :title="locked ? '取消锁定控制器自动隐藏' : '锁定控制器常驻显示'"
        @click="emit('toggle-lock')"
      >
        <Icon
          :icon="locked ? 'solar:lock-bold-duotone' : 'solar:lock-keyhole-minimalistic-unlocked-bold-duotone'"
          class="lock-icon"
        />
      </a-button>
    </div>
    <div ref="previewContainerRef" class="timeline-preview-zone" @mousemove="onTimelineMouseMove" @mouseleave="onTimelineMouseLeave">
      <a-slider
        :value="currentTime"
        :max="sliderMax"
        :disabled="disabled"
        title="拖动调整播放进度"
        :tooltip-open="false"
        @update:value="(value: number | [number, number]) => previewSeekWhilePaused(Array.isArray(value) ? Number(value[0]) : Number(value))"
        @change="(value: number | [number, number]) => emitSeek(Array.isArray(value) ? Number(value[0]) : Number(value))"
      />
      <div v-if="canShowPreview" class="timeline-preview-popup" :style="{ left: `${hoverLeft}px` }">
        <img :src="hoverImageSrc" :width="hoverImageWidth" :height="hoverImageHeight" alt="preview frame" />
        <span>{{ formatSeconds(hoverSeconds) }}</span>
      </div>
    </div>
    <div class="controls-row">
      <a-button
        class="control-btn primary"
        shape="circle"
        :disabled="disabled"
        :title="isPlaying ? '暂停播放' : '开始播放'"
        @click="isPlaying ? emitPause() : emit('play')"
      >
        {{ isPlaying ? "⏸" : "▶" }}
      </a-button>
      <a-button class="control-btn" shape="circle" :disabled="disabled" title="停止播放" @click="emit('stop')">
        ⏹
      </a-button>
      <a-select
        class="speed-select"
        :value="playbackRate"
        :options="speedOptions.map((value) => ({ value, label: `${value}x` }))"
        :disabled="disabled"
        title="调整播放倍速"
        @change="(value: number) => emit('change-rate', value)"
      />
      <a-button
        class="control-btn volume-btn"
        shape="circle"
        :disabled="disabled"
        :title="muted || volume <= 0 ? '取消静音' : '静音'"
        @click="emit('toggle-mute')"
      >
        <Icon :icon="volumeIcon" />
      </a-button>
      <a-slider
        class="volume-slider"
        :value="muted ? 0 : volume"
        :min="0"
        :max="1"
        :step="0.01"
        :tooltip-open="false"
        :disabled="disabled"
        title="调整音量"
        @change="
          (value: number | [number, number]) =>
            emit('change-volume', Array.isArray(value) ? Number(value[0]) : Number(value))
        "
      />
      <span class="time-label">{{ formatSeconds(currentTime) }} / {{ formatSeconds(duration) }}</span>
    </div>
  </section>
</template>

<style scoped>
.playback-controls {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px 16px 14px;
  border-radius: 12px;
  backdrop-filter: blur(18px);
  background: linear-gradient(
    to bottom,
    rgba(30, 30, 34, 0.42) 0%,
    rgba(20, 20, 24, 0.72) 35%,
    rgba(12, 12, 14, 0.8) 100%
  );
}

.top-row {
  display: flex;
  justify-content: flex-end;
}

.timeline-preview-zone {
  position: relative;
}

.timeline-preview-popup {
  position: absolute;
  bottom: calc(100% + 10px);
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  pointer-events: none;
  z-index: 6;
}

.timeline-preview-popup img {
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(0, 0, 0, 0.55);
}

.timeline-preview-popup span {
  padding: 1px 6px;
  border-radius: 4px;
  font-size: 11px;
  line-height: 16px;
  color: rgba(255, 255, 255, 0.92);
  background: rgba(0, 0, 0, 0.62);
}

.lock-btn {
  width: 28px;
  min-width: 28px;
  height: 28px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.08);
  color: #fff;
  transition: all 180ms ease;
}

.lock-btn:hover {
  border-color: rgba(255, 255, 255, 0.34);
  background: rgba(255, 255, 255, 0.18);
}

.lock-icon {
  font-size: 16px;
}

.controls-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.control-btn {
  width: 34px;
  min-width: 34px;
  height: 34px;
  border: none;
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
}

.control-btn.primary {
  background: rgba(255, 255, 255, 0.95);
  color: #0e0f13;
}

.time-label {
  margin-left: auto;
  color: rgba(255, 255, 255, 0.85);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.speed-select {
  width: 86px;
}

.volume-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.volume-slider {
  width: 90px;
}
</style>
