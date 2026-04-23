<script setup lang="ts">
import { ref } from "vue";
import { useMediaControls } from "@vueuse/core";

const props = defineProps<{
  source: string;
  loading: boolean;
}>();

const emit = defineEmits<{
  metadata: [duration: number];
  timeupdate: [position: number, duration: number];
  playing: [];
  pause: [];
  ended: [];
  "quick-open-local": [];
  "quick-open-url": [];
  "playback-error": [string];
}>();

const videoRef = ref<HTMLVideoElement | null>(null);
const { playing, currentTime, rate } = useMediaControls(videoRef);

async function playMedia() {
  if (!videoRef.value) {
    return;
  }
  try {
    await videoRef.value.play();
  } catch (error) {
    emit("playback-error", toPlaybackErrorMessage(error, props.source));
    throw error;
  }
}

function pauseMedia() {
  if (!videoRef.value) {
    return;
  }
  videoRef.value.pause();
}

function stopMedia() {
  if (!videoRef.value) {
    return;
  }
  videoRef.value.pause();
  currentTime.value = 0;
}

function seekTo(seconds: number) {
  currentTime.value = seconds;
}

function setRate(nextRate: number) {
  rate.value = nextRate;
}

defineExpose({
  playMedia,
  pauseMedia,
  stopMedia,
  seekTo,
  setRate,
  isPlaying: () => playing.value,
});

function toPlaybackErrorMessage(error: unknown, source: string) {
  if (error instanceof DOMException && error.name === "NotSupportedError") {
    return `当前播放器内核不支持该媒体编码或封装（可能是 H.265/HEVC）。请尝试其他源。${source}`;
  }
  if (error instanceof Error) {
    return `播放失败：${error.message}`;
  }
  return "播放失败：当前媒体源无法播放。";
}
</script>

<template>
  <section class="player-canvas">
    <video
      v-if="source"
      ref="videoRef"
      class="video"
      :src="source"
      @loadedmetadata="(event) => emit('metadata', (event.target as HTMLVideoElement).duration || 0)"
      @timeupdate="
        (event) =>
          emit(
            'timeupdate',
            (event.target as HTMLVideoElement).currentTime || 0,
            (event.target as HTMLVideoElement).duration || 0,
          )
      "
      @play="emit('playing')"
      @pause="emit('pause')"
      @ended="emit('ended')"
      @error="emit('playback-error', `播放失败：媒体资源加载异常。${source}`)"
    />
    <div v-else class="empty-actions">
      <a-empty description="请从 File 菜单打开本地文件或 URL">
        <template #default>
          <a-space>
            <a-button type="primary" @click="emit('quick-open-local')">打开本地文件</a-button>
            <a-button @click="emit('quick-open-url')">打开 URL</a-button>
          </a-space>
        </template>
      </a-empty>
    </div>
    <a-spin v-if="loading" class="busy-overlay" />
  </section>
</template>

<style scoped>
.player-canvas {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  background: #000;
  overflow: hidden;
}

.busy-overlay {
  position: absolute;
}

.empty-actions {
  padding: 20px;
}

.video {
  width: 100%;
  height: 100%;
  object-fit: contain;
}
</style>
