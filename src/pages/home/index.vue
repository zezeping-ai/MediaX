<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { clamp } from "lodash-es";
import MediaViewport from "./components/MediaViewport.vue";
import PlaybackControls from "./components/PlaybackControls.vue";
import { useMediaCenter } from "./composables/useMediaCenter";
import { usePlayerOverlayControls } from "./composables/usePlayerOverlayControls";
import { usePlaybackShortcuts } from "./composables/usePlaybackShortcuts";

const {
  playback,
  currentSource,
  isBusy,
  errorMessage,
  openLocalFileByDialog,
  requestOpenUrlInput,
  cancelOpenUrlInput,
  confirmOpenUrlInput,
  urlDialogVisible,
  urlInputValue,
  syncPosition,
  play,
  pause,
  stop,
  seek,
  seekPreview,
  requestPreviewFrame,
  setRate,
  setVolume,
  setMuted,
} = useMediaCenter();

const playbackRate = ref(1);
const volume = ref(1);
const muted = ref(false);
const playerErrorMessage = ref("");

const displayErrorMessage = computed(() => playerErrorMessage.value || errorMessage.value);
const hasSource = computed(() => Boolean(currentSource.value));
const {
  controlsVisible,
  controlsLocked,
  scheduleHideControls,
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

function increasePlaybackRate() {
  const nextRate = Math.min(3, Number((playbackRate.value + 0.25).toFixed(2)));
  void changePlaybackRate(nextRate);
}

function decreasePlaybackRate() {
  const nextRate = Math.max(0.25, Number((playbackRate.value - 0.25).toFixed(2)));
  void changePlaybackRate(nextRate);
}

async function handleVideoEnded() {
  await handleStop();
}

function handlePlaybackError(message: string) {
  playerErrorMessage.value = message;
}

watch(currentSource, () => {
  playerErrorMessage.value = "";
});

usePlaybackShortcuts({
  playback,
  onPlay: () => void handlePlay(),
  onPause: (positionSeconds) => void handlePause(positionSeconds),
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
  <main class="media-player-page">
    <section class="player-shell" @mousemove="markMouseActive" @mouseleave="scheduleHideControls">
      <MediaViewport
        :source="currentSource"
        :playback="playback"
        :loading="isBusy"
        @metadata="(duration) => syncPosition(0, duration)"
        @ended="handleVideoEnded"
        @quick-open-local="openLocalFileByDialog"
        @quick-open-url="requestOpenUrlInput"
        @playback-error="handlePlaybackError"
      />
      <PlaybackControls
        v-if="hasSource"
        class="overlay-controls"
        :class="{ 'overlay-controls-hidden': !controlsVisible }"
        :playback="playback"
        :playback-rate="playbackRate"
        :volume="volume"
        :muted="muted"
        :locked="controlsLocked"
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
        @toggle-mute="() => void toggleMute()"
        @toggle-lock="toggleLock"
      />
      <a-alert
        v-if="displayErrorMessage"
        class="overlay-error"
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
.media-player-page {
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: transparent;
}

.player-shell {
  position: relative;
  width: 100%;
  height: 100%;
}

:deep(.player-canvas) {
  width: 100%;
  height: 100%;
}

.overlay-controls {
  position: absolute;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  width: min(760px, calc(100vw - 32px));
  z-index: 3;
  opacity: 1;
  transition: opacity 220ms ease, transform 220ms ease;
}

.overlay-controls-hidden {
  opacity: 0;
  pointer-events: none;
  transform: translate(-50%, 12px);
}

.overlay-error {
  position: absolute;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  width: min(620px, calc(100vw - 32px));
  z-index: 4;
}

:deep(.ant-alert) {
  border-radius: 8px;
}

:deep(.ant-slider-track) {
  background: rgba(255, 255, 255, 0.88);
}

:deep(.ant-slider-handle::after) {
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.32);
  background: #fff;
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

