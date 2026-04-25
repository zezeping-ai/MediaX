<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useLocalStorage } from "@vueuse/core";
import { clamp } from "lodash-es";
import type { PlaybackQualityMode } from "@/modules/media-types";
import MediaViewport from "./components/MediaViewport/index.vue";
import PlaybackControls from "./components/PlaybackControls.vue";
import { useMediaCenter } from "./composables/useMediaCenter";
import { usePlayerOverlayControls } from "./composables/usePlayerOverlayControls";
import { usePlaybackShortcuts } from "./composables/usePlaybackShortcuts";
import { QUALITY_DOWNGRADE_LEVELS } from "./components/playbackControls.constants";
import { buildPlaybackQualityOptions } from "./components/playbackControls.utils";

const QUALITY_BASELINE_STORAGE_KEY = "mediax:quality-baseline-by-source";
const sourceHeightBaselineByPath = useLocalStorage<Record<string, number>>(
  QUALITY_BASELINE_STORAGE_KEY,
  {},
);

function readCachedSourceHeightBaseline(path: string): number | null {
  if (!path) {
    return null;
  }
  const value = sourceHeightBaselineByPath.value[path];
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
}

function writeCachedSourceHeightBaseline(path: string, height: number) {
  if (!path || !Number.isFinite(height) || height <= 0) {
    return;
  }
  const nextHeight = Math.round(height);
  const prevHeight = sourceHeightBaselineByPath.value[path];
  sourceHeightBaselineByPath.value[path] =
    typeof prevHeight === "number" && Number.isFinite(prevHeight) && prevHeight > 0
      ? Math.max(prevHeight, nextHeight)
      : nextHeight;
}

const {
  playback,
  currentSource,
  isBusy,
  errorMessage,
  recordingNoticeMessage,
  debugSnapshot,
  debugTimeline,
  mediaInfoSnapshot,
  openLocalFileByDialog,
  requestOpenUrlInput,
  cancelOpenUrlInput,
  confirmOpenUrlInput,
  urlDialogVisible,
  urlInputValue,
  urlPlaylist,
  openUrl,
  removeUrlFromPlaylist,
  clearUrlPlaylist,
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
  cacheRecording,
  cacheOutputPath,
  cacheOutputSizeBytes,
  cacheWriteSpeedBytesPerSecond,
  networkReadBytesPerSecond,
  cacheFinalizedOutputPath,
  effectiveDurationSeconds,
  toggleCacheRecording,
  metadataVideoHeight,
} = useMediaCenter();

function formatOpenedAt(timestamp: number) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return "未知时间";
  }
  return new Date(timestamp).toLocaleString();
}

async function handlePlayFromUrlPlaylist(url: string) {
  urlInputValue.value = url;
  await openUrl(url);
  urlDialogVisible.value = false;
}

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
  setControlsOverlayInteracting,
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
  const path = currentSource.value;
  sourceVideoHeightBaseline.value = path ? readCachedSourceHeightBaseline(path) : null;
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
  } else {
    sourceVideoHeightBaseline.value = Math.max(sourceVideoHeightBaseline.value, nextHeight);
  }
  if (currentSource.value && sourceVideoHeightBaseline.value !== null) {
    writeCachedSourceHeightBaseline(currentSource.value, sourceVideoHeightBaseline.value);
  }
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
        :network-read-bytes-per-second="networkReadBytesPerSecond"
        :cache-recording="cacheRecording"
        :cache-output-path="cacheOutputPath"
        :cache-output-size-bytes="cacheOutputSizeBytes"
        :cache-write-speed-bytes-per-second="cacheWriteSpeedBytesPerSecond"
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
        :cache-recording="cacheRecording"
        :cache-output-path="cacheOutputPath"
        :duration-seconds-override="effectiveDurationSeconds"
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
        @overlay-interaction-change="setControlsOverlayInteracting"
        @toggle-mute="() => void toggleMute()"
        @toggle-cache="toggleCacheRecording"
        @toggle-lock="toggleLock"
      />
      <a-alert
        v-if="cacheFinalizedOutputPath"
        class="absolute left-1/2 top-20 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
        type="success"
        :message="`缓存已生成：${cacheFinalizedOutputPath}`"
        show-icon
        banner
      />
      <a-alert
        v-if="recordingNoticeMessage"
        class="absolute left-1/2 top-4 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
        type="warning"
        :message="recordingNoticeMessage"
        show-icon
        banner
      />
      <a-alert
        v-if="displayErrorMessage"
        class="absolute left-1/2 top-16 z-40 w-[min(620px,calc(100vw-32px))] -translate-x-1/2"
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
        <a-space direction="vertical" class="w-full" :size="12">
          <a-input
            v-model:value="urlInputValue"
            placeholder="请输入视频 URL（http/https）"
            allow-clear
            @press-enter="confirmOpenUrlInput"
          />
          <div class="flex items-center justify-between">
            <span class="text-xs opacity-70">播放列表（最近优先）</span>
            <a-button
              v-if="urlPlaylist.length"
              size="small"
              danger
              type="text"
              @click="clearUrlPlaylist"
            >
              一键清空
            </a-button>
          </div>
          <a-empty v-if="!urlPlaylist.length" description="暂无历史 URL" />
          <a-list v-else size="small" :data-source="urlPlaylist">
            <template #renderItem="{ item }">
              <a-list-item class="overflow-hidden">
                <div class="min-w-0 w-full space-y-1 overflow-hidden">
                  <button
                    class="block min-w-0 w-full cursor-pointer bg-transparent p-0 text-left"
                    type="button"
                    :title="item.url"
                    @click="urlInputValue = item.url"
                  >
                    <span
                      class="block min-w-0 w-full overflow-hidden text-ellipsis whitespace-nowrap text-xs text-[rgba(255,255,255,0.85)]"
                    >
                      {{ item.url }}
                    </span>
                  </button>
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-xs opacity-70">{{ formatOpenedAt(item.lastOpenedAt) }}</span>
                    <a-space :size="4">
                      <a-button
                        size="small"
                        type="link"
                        :disabled="isBusy"
                        @click="handlePlayFromUrlPlaylist(item.url)"
                      >
                        播放
                      </a-button>
                      <a-button
                        size="small"
                        danger
                        type="text"
                        @click="removeUrlFromPlaylist(item.url)"
                      >
                        删除
                      </a-button>
                    </a-space>
                  </div>
                </div>
              </a-list-item>
            </template>
          </a-list>
        </a-space>
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

