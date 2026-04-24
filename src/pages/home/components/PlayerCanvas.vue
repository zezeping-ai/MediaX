<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { startMediaStream, stopMediaStream } from "../../../modules/media";

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

type MediaFramePayload = {
  width: number;
  height: number;
  position_seconds: number;
  rgba: number[];
};

type MediaMetadataPayload = {
  width: number;
  height: number;
  fps: number;
  duration_seconds: number;
};

type MediaErrorPayload = {
  code: string;
  message: string;
};

const FRAME_EVENT = "media://frame";
const METADATA_EVENT = "media://metadata";
const ERROR_EVENT = "media://error";

const canvasRef = ref<HTMLCanvasElement | null>(null);
const isPlayingRef = ref(false);
const durationRef = ref(0);
let unlistenFrameEvent: UnlistenFn | null = null;
let unlistenMetadataEvent: UnlistenFn | null = null;
let unlistenErrorEvent: UnlistenFn | null = null;

async function playMedia() {
  if (!props.source) {
    return;
  }
  try {
    await startMediaStream(props.source);
    isPlayingRef.value = true;
    emit("playing");
  } catch (error) {
    emit("playback-error", toPlaybackErrorMessage(error));
    throw error;
  }
}

async function pauseMedia() {
  await stopMediaStream();
  isPlayingRef.value = false;
  emit("pause");
}

async function stopMedia() {
  await stopMediaStream();
  isPlayingRef.value = false;
  clearCanvas();
  emit("pause");
}

function seekTo(seconds: number) {
  // First version: seek is controlled by backend state.
  void seconds;
}

function setRate(nextRate: number) {
  // First version: playback rate will be wired with decoder clock later.
  void nextRate;
}

defineExpose({
  playMedia,
  pauseMedia,
  stopMedia,
  seekTo,
  setRate,
  isPlaying: () => isPlayingRef.value,
});

function toPlaybackErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return `播放失败：${error.message}`;
  }
  return "播放失败：当前媒体源无法播放。";
}

function drawFrame(payload: MediaFramePayload) {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }
  if (canvas.width !== payload.width || canvas.height !== payload.height) {
    canvas.width = payload.width;
    canvas.height = payload.height;
  }
  const context = canvas.getContext("2d");
  if (!context) {
    return;
  }
  const image = new ImageData(Uint8ClampedArray.from(payload.rgba), payload.width, payload.height);
  context.putImageData(image, 0, 0);
  emit("timeupdate", payload.position_seconds, durationRef.value);
}

function clearCanvas() {
  const canvas = canvasRef.value;
  if (!canvas) {
    return;
  }
  const context = canvas.getContext("2d");
  if (!context) {
    return;
  }
  context.clearRect(0, 0, canvas.width, canvas.height);
}

onMounted(async () => {
  unlistenFrameEvent = await listen<MediaFramePayload>(FRAME_EVENT, (event) => {
    drawFrame(event.payload);
  });
  unlistenMetadataEvent = await listen<MediaMetadataPayload>(METADATA_EVENT, (event) => {
    durationRef.value = event.payload.duration_seconds;
    emit("metadata", event.payload.duration_seconds);
  });
  unlistenErrorEvent = await listen<MediaErrorPayload>(ERROR_EVENT, (event) => {
    emit("playback-error", `${event.payload.code}: ${event.payload.message}`);
  });
});

onBeforeUnmount(() => {
  void stopMediaStream();
  unlistenFrameEvent?.();
  unlistenFrameEvent = null;
  unlistenMetadataEvent?.();
  unlistenMetadataEvent = null;
  unlistenErrorEvent?.();
  unlistenErrorEvent = null;
});

watch(
  () => props.source,
  (nextSource) => {
    void stopMediaStream();
    isPlayingRef.value = false;
    durationRef.value = 0;
    clearCanvas();
    if (!nextSource) {
      emit("ended");
    }
  },
);
</script>

<template>
  <section class="player-canvas">
    <canvas v-if="source" ref="canvasRef" class="video" />
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
