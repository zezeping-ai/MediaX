<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { clamp } from "lodash-es";
import type { PlaybackQualityMode } from "@/modules/media-types";
import MediaViewport from "./components/MediaViewport/index.vue";
import PlaybackControls from "./components/PlaybackControls.vue";
import { useMediaCenter } from "./composables/useMediaCenter";
import { usePlayerOverlayControls } from "./composables/usePlayerOverlayControls";
import { usePlaybackShortcuts } from "./composables/usePlaybackShortcuts";
import { QUALITY_DOWNGRADE_LEVELS } from "./components/playbackControls.constants";
import { buildPlaybackQualityOptions } from "./components/playbackControls.utils";

const {
  playback,
  currentSource,
  isBusy,
  errorMessage,
  debugSnapshot,
  debugTimeline,
  mediaInfoSnapshot,
  openLocalFileByDialog,
  requestOpenUrlInput,
  cancelOpenUrlInput,
  confirmOpenUrlInput,
  urlDialogVisible,
  urlInputValue,
  play,
  pause,
  stop,
  seek,
  seekPreview,
  requestPreviewFrame,
  setRate,
  setVolume,
  setMuted,
  setQuality,
  metadataVideoHeight,
} = useMediaCenter();

const playbackRate = ref(1);
const volume = ref(1);
const muted = ref(false);
const selectedQuality = ref("source");
const sourceVideoHeightBaseline = ref<number | null>(null);
const playerErrorMessage = ref("");

const displayErrorMessage = computed(() => playerErrorMessage.value || errorMessage.value);
const hasSource = computed(() => Boolean(currentSource.value));
const adaptiveQualitySupported = computed(() => Boolean(playback.value?.adaptive_quality_supported));
const playbackQualityOptions = computed(() =>
  buildPlaybackQualityOptions(
    sourceVideoHeightBaseline.value,
    QUALITY_DOWNGRADE_LEVELS,
    adaptiveQualitySupported.value,
    selectedQuality.value,
  ),
);
const {
  controlsVisible,
  controlsLocked,
  hideControlsImmediately,
  markMouseActive,
  toggleLock,
  onControlsMouseEnter,
  onControlsMouseLeave,
} = usePlayerOverlayControls({
  hasSource,
  isBusy,
});

async function handlePlay() {
  await play();
  playerErrorMessage.value = "";
}

async function handlePause(positionSeconds?: number) {
  if (typeof positionSeconds === "number" && Number.isFinite(positionSeconds)) {
    await seek(positionSeconds);
  }
  await pause();
}

async function handleStop() {
  await stop();
}

async function handleSeek(seconds: number) {
  await seek(seconds);
}

async function handleSeekPreview(seconds: number) {
  await seekPreview(seconds);
}

async function handleRequestPreviewFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
  return requestPreviewFrame(positionSeconds, maxWidth, maxHeight);
}

async function changePlaybackRate(rate: number) {
  playbackRate.value = rate;
  await setRate(rate);
}

async function changeVolume(nextVolume: number) {
  const normalized = clamp(nextVolume, 0, 1);
  volume.value = normalized;
  muted.value = normalized <= 0;
  await setVolume(normalized);
}

async function toggleMute() {
  muted.value = !muted.value;
  await setMuted(muted.value);
}

async function changeQuality(nextQuality: string) {
  const nextMode = nextQuality as PlaybackQualityMode;
  selectedQuality.value = nextQuality;
  await setQuality(nextMode);
}

function increasePlaybackRate() {
  const nextRate = Math.min(3, Number((playbackRate.value + 0.1).toFixed(1)));
  void changePlaybackRate(nextRate);
}

function decreasePlaybackRate() {
  const nextRate = Math.max(0.1, Number((playbackRate.value - 0.1).toFixed(1)));
  void changePlaybackRate(nextRate);
}

async function handleVideoEnded() {
  await handleStop();
}

watch(currentSource, () => {
  playerErrorMessage.value = "";
  selectedQuality.value = "source";
  sourceVideoHeightBaseline.value = null;
});

watch(playback, (value) => {
  if (!value) {
    selectedQuality.value = "source";
    return;
  }
  selectedQuality.value = value.quality_mode ?? "source";
});

watch(metadataVideoHeight, (nextHeight) => {
  if (typeof nextHeight !== "number" || !Number.isFinite(nextHeight) || nextHeight <= 0) {
    return;
  }
  if (sourceVideoHeightBaseline.value === null) {
    sourceVideoHeightBaseline.value = nextHeight;
    return;
  }
  sourceVideoHeightBaseline.value = Math.max(sourceVideoHeightBaseline.value, nextHeight);
});

usePlaybackShortcuts({
  playback,
  onPlay: () => void handlePlay(),
  onPause: (positionSeconds) => void handlePause(positionSeconds),
  onSeek: (positionSeconds) => void handleSeek(positionSeconds),
  onResetRate: () => void changePlaybackRate(1),
  onIncreaseRate: increasePlaybackRate,
  onDecreaseRate: decreasePlaybackRate,
});

watch(playback, (value) => {
  if (!value) {
    return;
  }
  playbackRate.value = value.playback_rate ?? 1;
});
</script>

<template>
  <main class="h-screen w-screen overflow-hidden bg-transparent">
    <section
      class="relative h-full w-full"
      @mousemove="markMouseActive"
      @mouseleave="hideControlsImmediately"
    >
      <MediaViewport
        :source="currentSource"
        :playback="playback"
        :loading="isBusy"
        :debug-snapshot="debugSnapshot"
        :debug-timeline="debugTimeline"
        :media-info-snapshot="mediaInfoSnapshot"
        @ended="handleVideoEnded"
        @quick-open-local="openLocalFileByDialog"
        @quick-open-url="requestOpenUrlInput"
      />
      <PlaybackControls
        v-if="hasSource"
        class="absolute bottom-0 left-1/2 z-30 w-[min(760px,calc(100vw-32px))] -translate-x-1/2 opacity-100 transition-[opacity,transform] duration-300 ease-out will-change-transform"
        :class="!controlsVisible ? 'pointer-events-none translate-y-[120%] opacity-0' : ''"
        :playback="playback"
        :playback-rate="playbackRate"
        :volume="volume"
        :muted="muted"
        :locked="controlsLocked"
        :quality-options="playbackQualityOptions"
        :selected-quality="selectedQuality"
        :disabled="!currentSource || isBusy"
        :request-preview-frame="handleRequestPreviewFrame"
        @mouseenter="onControlsMouseEnter"
        @mouseleave="onControlsMouseLeave"
        @mousemove="markMouseActive"
        @play="handlePlay"
        @pause="(position) => handlePause(position)"
        @stop="handleStop"
        @seek="handleSeek"
        @seek-preview="handleSeekPreview"
        @change-rate="(value) => void changePlaybackRate(value)"
        @change-volume="(value) => void changeVolume(value)"
        @change-quality="(value) => void changeQuality(value)"
        @toggle-mute="() => void toggleMute()"
        @toggle-lock="toggleLock"
      />
      <a-alert
        v-if="displayErrorMessage"
        class="absolute left-1/2 top-4 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
        type="error"
        :message="displayErrorMessage"
        show-icon
        banner
      />
      <a-modal
        v-model:open="urlDialogVisible"
        title="打开 URL"
        ok-text="开始播放"
        cancel-text="取消"
        :confirm-loading="isBusy"
        @ok="confirmOpenUrlInput"
        @cancel="cancelOpenUrlInput"
      >
        <a-input
          v-model:value="urlInputValue"
          placeholder="请输入视频 URL（http/https）"
          allow-clear
          @press-enter="confirmOpenUrlInput"
        />
      </a-modal>
    </section>
  </main>
</template>

<style scoped>
/* .media-player-page removed (Tailwind) */

/* .player-shell removed (Tailwind) */

/* :deep(.player-canvas) sizing handled by component */

/* .overlay-controls removed (Tailwind) */

/* .overlay-controls-hidden removed (Tailwind) */

/* .overlay-error removed (Tailwind) */

:deep(.ant-alert) {
  border-radius: 8px;
}

:deep(.ant-empty-description) {
  color: rgba(255, 255, 255, 0.75);
}

:deep(.ant-alert-message) {
  color: #fff;
}

:deep(.ant-alert-error) {
  background: rgba(255, 77, 79, 0.2);
  border: 1px solid rgba(255, 77, 79, 0.35);
}

:deep(.video) {
  object-fit: contain;
}
</style>

