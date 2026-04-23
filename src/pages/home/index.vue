<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useEventListener } from "@vueuse/core";
import PlayerCanvas from "./components/PlayerCanvas.vue";
import PlaybackControls from "./components/PlaybackControls.vue";
import { useMediaCenter } from "./composables/useMediaCenter";

type PlayerCanvasExposed = {
  playMedia: () => Promise<void>;
  pauseMedia: () => void;
  stopMedia: () => void;
  seekTo: (seconds: number) => void;
  setRate: (rate: number) => void;
  isPlaying: () => boolean;
};

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
  setRate,
} = useMediaCenter();

const canvasRef = ref<PlayerCanvasExposed | null>(null);
const playbackRate = ref(1);
const controlsVisible = ref(true);
const controlsLocked = ref(false);
const playerErrorMessage = ref("");
let hideControlsTimer: number | null = null;

const displayErrorMessage = computed(() => playerErrorMessage.value || errorMessage.value);

async function handlePlay() {
  try {
    await canvasRef.value?.playMedia();
    await play();
    playerErrorMessage.value = "";
  } catch {
    // 播放失败已通过 playback-error 事件展示
  }
}

async function handlePause() {
  canvasRef.value?.pauseMedia();
  await pause();
}

async function handleStop() {
  canvasRef.value?.stopMedia();
  await stop();
}

async function handleSeek(seconds: number) {
  canvasRef.value?.seekTo(seconds);
  await seek(seconds);
}

async function changePlaybackRate(rate: number) {
  playbackRate.value = rate;
  canvasRef.value?.setRate(rate);
  await setRate(rate);
}

function increasePlaybackRate() {
  const nextRate = Math.min(3, Number((playbackRate.value + 0.25).toFixed(2)));
  void changePlaybackRate(nextRate);
}

function decreasePlaybackRate() {
  const nextRate = Math.max(0.25, Number((playbackRate.value - 0.25).toFixed(2)));
  void changePlaybackRate(nextRate);
}

async function handleVideoPlaying() {
  playerErrorMessage.value = "";
  await play();
}

async function handleVideoPause() {
  await pause();
}

async function handleVideoEnded() {
  await handleStop();
}

function handlePlaybackError(message: string) {
  playerErrorMessage.value = message;
}

const shouldKeepControlsVisible = computed(
  () => controlsLocked.value || !currentSource.value || isBusy.value,
);

function showControls() {
  controlsVisible.value = true;
}

function clearHideTimer() {
  if (hideControlsTimer !== null) {
    window.clearTimeout(hideControlsTimer);
    hideControlsTimer = null;
  }
}

function scheduleHideControls() {
  clearHideTimer();
  if (shouldKeepControlsVisible.value) {
    showControls();
    return;
  }
  hideControlsTimer = window.setTimeout(() => {
    controlsVisible.value = false;
  }, 1800);
}

function handleMouseActivity() {
  showControls();
  scheduleHideControls();
}

function toggleControlsLock() {
  controlsLocked.value = !controlsLocked.value;
  if (controlsLocked.value) {
    clearHideTimer();
    showControls();
    return;
  }
  scheduleHideControls();
}

watch(shouldKeepControlsVisible, (keepVisible) => {
  if (keepVisible) {
    clearHideTimer();
    showControls();
    return;
  }
  scheduleHideControls();
});

watch(currentSource, () => {
  playerErrorMessage.value = "";
});

onBeforeUnmount(() => {
  clearHideTimer();
});

useEventListener(window, "keydown", (event: KeyboardEvent) => {
  if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
    return;
  }
  if (event.code === "Space") {
    event.preventDefault();
    if (playback.value?.status === "playing") {
      void handlePause();
    } else {
      void handlePlay();
    }
    return;
  }
  if (event.key === "]") {
    event.preventDefault();
    increasePlaybackRate();
    return;
  }
  if (event.key === "[") {
    event.preventDefault();
    decreasePlaybackRate();
    return;
  }
  if (event.key === "0") {
    event.preventDefault();
    void changePlaybackRate(1);
  }
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
    <section class="player-shell" @mousemove="handleMouseActivity" @mouseleave="scheduleHideControls">
      <PlayerCanvas
        ref="canvasRef"
        :source="currentSource"
        :loading="isBusy"
        @metadata="(duration) => syncPosition(0, duration)"
        @timeupdate="(position, duration) => syncPosition(position, duration)"
        @playing="handleVideoPlaying"
        @pause="handleVideoPause"
        @ended="handleVideoEnded"
        @quick-open-local="openLocalFileByDialog"
        @quick-open-url="requestOpenUrlInput"
        @playback-error="handlePlaybackError"
      />
      <PlaybackControls
        class="overlay-controls"
        :class="{ 'overlay-controls-hidden': !controlsVisible }"
        :playback="playback"
        :playback-rate="playbackRate"
        :locked="controlsLocked"
        :disabled="!currentSource || isBusy"
        @mouseenter="showControls"
        @mousemove="handleMouseActivity"
        @play="handlePlay"
        @pause="handlePause"
        @stop="handleStop"
        @seek="handleSeek"
        @change-rate="(value) => void changePlaybackRate(value)"
        @toggle-lock="toggleControlsLock"
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
  background: #000;
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

