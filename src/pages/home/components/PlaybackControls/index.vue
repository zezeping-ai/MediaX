<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { PlaybackState, PreviewFrame } from "@/modules/media-types";
import { usePlaybackTimelineState } from "../../composables/usePlaybackTimelineState";
import PlaybackCenterControls from "./PlaybackCenterControls.vue";
import PlaybackSideActions from "./PlaybackSideActions.vue";
import PlaybackTimeline from "./PlaybackTimeline.vue";
import { type PlaybackQualityOption } from "./playbackControlsUtils";

const props = defineProps<{
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  volume: number;
  muted: boolean;
  locked: boolean;
  cacheRecording: boolean;
  cacheOutputPath: string;
  durationSecondsOverride: number;
  qualityOptions: PlaybackQualityOption[];
  selectedQuality: string;
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
  "change-quality": [string];
  "overlay-interaction-change": [boolean];
  "toggle-mute": [];
  "toggle-cache": [];
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
const duration = computed(() => {
  const base = props.playback?.duration_seconds ?? 0;
  const override = props.durationSecondsOverride ?? 0;
  const normalizedBase = Number.isFinite(base) ? Math.max(0, base) : 0;
  const normalizedOverride = Number.isFinite(override) ? Math.max(0, override) : 0;
  return Math.max(normalizedBase, normalizedOverride);
});
const canSeek = computed(() => {
  const playback = props.playback;
  if (!playback || !playback.current_path) {
    return false;
  }
  // For streams that don't support seeking (commonly some live/m3u8),
  // backend reports duration_seconds as 0. Disable timeline interaction to avoid bad seeks.
  return Number.isFinite(playback.duration_seconds) && playback.duration_seconds > 0;
});
const timelineDisabled = computed(() => props.disabled || !canSeek.value);
const timelineTitle = computed(() =>
  timelineDisabled.value ? "当前流不支持跳转进度" : "拖动调整播放进度",
);
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
const qualityLabel = computed(() => {
  const matched = props.qualityOptions.find((option) => option.key === props.selectedQuality);
  return matched?.label ?? "原画";
});

// 线性小图标：比 duotone 更轻，与音量区图标体量接近
const lockIcon = computed(() => (props.locked ? "lucide:lock" : "lucide:lock-open"));
const cacheIcon = computed(() => (props.cacheRecording ? "lucide:database-zap" : "lucide:database"));
const speedDropdownOpen = ref(false);
const qualityDropdownOpen = ref(false);

watch(
  () => speedDropdownOpen.value || qualityDropdownOpen.value,
  (open) => {
    emit("overlay-interaction-change", open);
  },
);

function handleSpeedMenuClick({ key }: { key: string | number }) {
  speedDropdownOpen.value = false;
  emit("change-rate", Number(key));
}

function handleQualityMenuClick({ key }: { key: string | number }) {
  qualityDropdownOpen.value = false;
  emit("change-quality", String(key));
}

function handleProgressPreviewUpdate(value: number | [number, number]) {
  if (!canSeek.value) {
    return;
  }
  previewSeekWhilePaused(normalizeSliderValue(value));
}

function handleProgressCommit(value: number | [number, number]) {
  if (!canSeek.value) {
    return;
  }
  commitSeek(normalizeSliderValue(value));
}

function handleVolumeChange(value: number | [number, number]) {
  emit("change-volume", normalizeSliderValue(value));
}

onBeforeUnmount(() => {
  cancelPreviewSeek();
  emit("overlay-interaction-change", false);
});
</script>

<template>
  <section
    class="w-full overflow-visible rounded-t-2xl rounded-b-none border border-white/10 bg-[linear-gradient(180deg,rgba(0,0,0,0.25)_0%,rgba(0,0,0,0.35)_100%)] shadow-[0_18px_60px_rgba(0,0,0,0.55)] backdrop-blur-2xl"
  >
    <div class="px-3.5 pb-2 pt-2.5">
      <PlaybackTimeline
        :current-time="currentTime"
        :duration="duration"
        :slider-max="sliderMax"
        :timeline-disabled="timelineDisabled"
        :timeline-title="timelineTitle"
        :source-key="playback?.current_path ?? ''"
        :request-preview-frame="requestPreviewFrame"
        @preview="handleProgressPreviewUpdate"
        @commit="handleProgressCommit"
      />

      <div
        class="mt-1 grid grid-cols-[40px_minmax(0,1fr)_40px_40px] items-center gap-2 max-[720px]:grid-cols-[34px_minmax(0,1fr)_34px_34px]"
      >
        <div aria-hidden="true" />
        <PlaybackCenterControls
          :current-time="currentTime"
          :duration="duration"
          :disabled="disabled"
          :is-playing="isPlaying"
          :playback-rate="playbackRate"
          :selected-quality="selectedQuality"
          :quality-label="qualityLabel"
          :quality-options="qualityOptions"
          :muted="muted"
          :volume="volume"
          :volume-icon="volumeIcon"
          :speed-dropdown-open="speedDropdownOpen"
          :quality-dropdown-open="qualityDropdownOpen"
          @play="emit('play')"
          @pause="emit('pause', currentTime)"
          @stop="emit('stop')"
          @toggle-speed-open="speedDropdownOpen = $event"
          @toggle-quality-open="qualityDropdownOpen = $event"
          @change-speed="handleSpeedMenuClick({ key: $event })"
          @change-quality="handleQualityMenuClick({ key: $event })"
          @toggle-mute="emit('toggle-mute')"
          @change-volume="handleVolumeChange"
        />

        <PlaybackSideActions
          :cache-recording="cacheRecording"
          :locked="locked"
          :cache-icon="cacheIcon"
          :lock-icon="lockIcon"
          @toggle-cache="emit('toggle-cache')"
          @toggle-lock="emit('toggle-lock')"
        />
      </div>
    </div>
  </section>
</template>
