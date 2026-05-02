<script setup lang="ts">
import { computed } from "vue";
import {
  type MediaAudioMeterPayload,
  type MediaLyricLine,
  type PlaybackChannelRouting,
  type PlaybackState,
} from "@/modules/media-types";
import {
  startMainWindowDragging,
  toggleMainWindowFullscreen,
} from "@/modules/media-player/windowCommands";
import AudioLyricsOverlay from "./AudioLyricsOverlay";
import TransferStatusOverlay from "./TransferStatusOverlay.vue";

const props = defineProps<{
  source: string;
  pendingSource: string;
  controlsVisible: boolean;
  loading: boolean;
  playback: PlaybackState | null;
  latestAudioMeter: MediaAudioMeterPayload | null;
  metadataMediaKind: "video" | "audio";
  metadataTitle: string;
  metadataArtist: string;
  metadataAlbum: string;
  metadataHasCoverArt: boolean;
  metadataLyrics: MediaLyricLine[];
  setLeftChannelVolume: (volume: number) => Promise<void>;
  setRightChannelVolume: (volume: number) => Promise<void>;
  setLeftChannelMuted: (muted: boolean) => Promise<void>;
  setRightChannelMuted: (muted: boolean) => Promise<void>;
  setChannelRouting: (routing: PlaybackChannelRouting) => Promise<void>;
  networkReadBytesPerSecond: number | null;
  networkSustainRatio: number | null;
  cacheRecording: boolean;
  cacheOutputPath: string;
  cacheOutputSizeBytes: number | null;
  cacheWriteSpeedBytesPerSecond: number | null;
}>();

const emit = defineEmits<{
  ended: [];
  "quick-open-local": [];
  "quick-open-url": [];
}>();

const hasVideoPresentationSignals = computed(() =>
  props.playback?.media_kind === "video" || props.metadataMediaKind === "video",
);
const hasAudioPresentationSignals = computed(() =>
  Boolean(
    props.playback?.media_kind === "audio"
    || props.metadataMediaKind === "audio"
    || (!hasVideoPresentationSignals.value && props.metadataHasCoverArt)
  ),
);
const effectiveMediaKind = computed(() => {
  if (hasAudioPresentationSignals.value) {
    return "audio";
  }
  return "video";
});

const WINDOW_DRAG_BLOCK_SELECTORS = [
  "button",
  "input",
  "textarea",
  "select",
  "label",
  "a",
  "[role='button']",
  "[role='slider']",
  "[role='dialog']",
  "[data-no-window-drag='true']",
  ".ant-btn",
  ".ant-slider",
  ".ant-select",
  ".ant-dropdown",
  ".ant-modal",
  ".ant-empty",
].join(", ");

function isInteractiveTarget(target: EventTarget | null) {
  return target instanceof Element && Boolean(target.closest(WINDOW_DRAG_BLOCK_SELECTORS));
}

async function handleViewportDoubleClick(event: MouseEvent) {
  if (isInteractiveTarget(event.target)) {
    return;
  }
  await toggleMainWindowFullscreen();
}

async function handleViewportPointerDown(event: PointerEvent) {
  if (event.button !== 0 || !event.isPrimary || event.detail > 1) {
    return;
  }
  if (isInteractiveTarget(event.target)) {
    return;
  }
  await startMainWindowDragging();
}
</script>

<template>
  <section
    class="relative flex h-full items-center justify-center overflow-hidden bg-transparent"
    @dblclick="handleViewportDoubleClick"
    @pointerdown="handleViewportPointerDown"
  >
    <div v-if="source" class="h-full w-full" />
    <div v-else class="p-5">
      <a-empty description="请从 File 菜单打开本地文件或 URL">
        <template #default>
          <a-space>
            <a-button type="primary" @click="emit('quick-open-local')">打开本地文件</a-button>
            <a-button @click="emit('quick-open-url')">打开 URL</a-button>
          </a-space>
        </template>
      </a-empty>
    </div>
    <AudioLyricsOverlay
      :media-kind="effectiveMediaKind"
      :playback="playback"
      :audio-meter="latestAudioMeter"
      :lyrics="metadataLyrics"
      :title="metadataTitle"
      :artist="metadataArtist"
      :album="metadataAlbum"
      :has-cover-art="metadataHasCoverArt"
      :set-left-channel-volume="setLeftChannelVolume"
      :set-right-channel-volume="setRightChannelVolume"
      :set-left-channel-muted="setLeftChannelMuted"
      :set-right-channel-muted="setRightChannelMuted"
    />
    <TransferStatusOverlay
      :source="source"
      :network-read-bytes-per-second="networkReadBytesPerSecond"
      :network-sustain-ratio="networkSustainRatio"
      :cache-recording="cacheRecording"
      :cache-output-path="cacheOutputPath"
      :cache-output-size-bytes="cacheOutputSizeBytes"
      :cache-write-speed-bytes-per-second="cacheWriteSpeedBytesPerSecond"
    />
  </section>
</template>

<style scoped>
</style>
