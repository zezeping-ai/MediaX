<script setup lang="ts">
import { computed } from "vue";
import { Icon } from "@iconify/vue";
import type { PlaybackState } from "../../../modules/media";

const props = defineProps<{
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  locked: boolean;
}>();

const emit = defineEmits<{
  play: [];
  pause: [];
  stop: [];
  seek: [number];
  "change-rate": [number];
  "toggle-lock": [];
}>();

const currentTime = computed(() => props.playback?.position_seconds ?? 0);
const duration = computed(() => props.playback?.duration_seconds ?? 0);
const isPlaying = computed(() => props.playback?.status === "playing");
const speedOptions = [0.25, 0.5, 0.75, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3];

function formatSeconds(value: number) {
  const safeValue = Math.max(0, Math.floor(value || 0));
  const minutes = Math.floor(safeValue / 60);
  const seconds = safeValue % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
}
</script>

<template>
  <section class="playback-controls">
    <div class="top-row">
      <a-button class="lock-btn" shape="circle" @click="emit('toggle-lock')">
        <Icon
          :icon="locked ? 'solar:lock-bold-duotone' : 'solar:lock-keyhole-minimalistic-unlocked-bold-duotone'"
          class="lock-icon"
        />
      </a-button>
    </div>
    <a-slider
      :value="currentTime"
      :max="Math.max(duration, 1)"
      :disabled="disabled"
      :tooltip-open="false"
      @change="
        (value: number | [number, number]) =>
          emit('seek', Array.isArray(value) ? Number(value[0]) : Number(value))
      "
    />
    <div class="controls-row">
      <a-button class="control-btn" shape="circle" :disabled="disabled" @click="emit('seek', 0)">
        ⏮
      </a-button>
      <a-button
        class="control-btn primary"
        shape="circle"
        :disabled="disabled"
        @click="isPlaying ? emit('pause') : emit('play')"
      >
        {{ isPlaying ? "⏸" : "▶" }}
      </a-button>
      <a-button class="control-btn" shape="circle" :disabled="disabled" @click="emit('stop')">
        ⏹
      </a-button>
      <a-select
        class="speed-select"
        :value="playbackRate"
        :options="speedOptions.map((value) => ({ value, label: `${value}x` }))"
        :disabled="disabled"
        @change="(value: number) => emit('change-rate', value)"
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
</style>
