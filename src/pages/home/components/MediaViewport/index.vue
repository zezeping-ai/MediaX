<script setup lang="ts">
import { computed } from "vue";
import {
  type LyricsCandidateSummary,
  type MediaAudioMeterPayload,
  type MediaLyricLine,
  type MediaSnapshot,
  type PlaybackChannelRouting,
  type PlaybackState,
} from "@/modules/media-types";
import {
  startMainWindowDragging,
  toggleMainWindowFullscreen,
} from "@/modules/media-player/windowCommands";
import AudioLyricPanel from "./AudioLyricsOverlay";
import TransferStatusOverlay from "./TransferStatusOverlay.vue";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";

const props = defineProps<{
  source: string;
  pendingSource: string;
  initialized: boolean;
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
  metadataLyricsSource: string | null;
  metadataLyricsCandidateId: string | null;
  metadataLyricsCandidates: LyricsCandidateSummary[];
  metadataLyricsFetching: boolean;
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
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
}>();

const emit = defineEmits<{
  ended: [];
  "quick-open-local": [];
  "quick-open-url": [];
}>();

const hasAudioPresentationSignals = computed(() =>
  Boolean(
    props.playback?.media_kind === "audio"
    || props.metadataMediaKind === "audio"
  ),
);
const effectiveMediaKind = computed(() => {
  if (hasAudioPresentationSignals.value) {
    return "audio";
  }
  return "video";
});
const playbackStatus = computed(() => props.playback?.status ?? "idle");
const isTerminalPlaybackState = computed(() =>
  playbackStatus.value === "idle" || playbackStatus.value === "stopped",
);
const hasRenderableSource = computed(() =>
  Boolean(props.source || props.pendingSource || props.playback?.current_path),
);
const showPlaybackSurface = computed(() =>
  hasRenderableSource.value || !isTerminalPlaybackState.value,
);
const showEmptySourceActions = computed(() =>
  props.initialized && !props.loading && !hasRenderableSource.value && isTerminalPlaybackState.value,
);

const { emptyStateBackdrop, emptyStatePanel } = useAppSurfaceTheme();

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

async function handleViewportMouseDown(event: MouseEvent) {
  if (event.button !== 0 || event.detail > 1) {
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
    data-tauri-drag-region
    class="relative flex h-full items-center justify-center overflow-hidden bg-transparent"
    @dblclick.capture="handleViewportDoubleClick"
    @mousedown.capture="handleViewportMouseDown"
  >
    <Transition
      mode="out-in"
      enter-active-class="transition-opacity duration-200 ease-out"
      leave-active-class="transition-opacity duration-180 ease-in"
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
    >
      <div v-if="showPlaybackSurface" key="playback-surface" class="h-full w-full" />
      <div
        v-else-if="showEmptySourceActions"
        key="empty-actions"
        class="absolute inset-0 z-10 flex items-center justify-center p-5"
        :class="emptyStateBackdrop"
      >
        <div :class="emptyStatePanel">
          <a-empty description="请从 File 菜单打开本地文件或 URL">
            <template #default>
              <a-space>
                <a-button type="primary" @click="emit('quick-open-local')">打开本地文件</a-button>
                <a-button @click="emit('quick-open-url')">打开 URL</a-button>
              </a-space>
            </template>
          </a-empty>
        </div>
      </div>
    </Transition>
    <AudioLyricPanel
      :media-kind="effectiveMediaKind"
      :playback="playback"
      :audio-meter="latestAudioMeter"
      :lyrics="metadataLyrics"
      :lyrics-source="metadataLyricsSource"
      :lyrics-candidate-id="metadataLyricsCandidateId"
      :lyrics-candidates="metadataLyricsCandidates"
      :lyrics-fetching="metadataLyricsFetching"
      :title="metadataTitle"
      :artist="metadataArtist"
      :album="metadataAlbum"
      :has-cover-art="metadataHasCoverArt"
      :set-left-channel-volume="setLeftChannelVolume"
      :set-right-channel-volume="setRightChannelVolume"
      :set-left-channel-muted="setLeftChannelMuted"
      :set-right-channel-muted="setRightChannelMuted"
      :update-playback-snapshot="props.updatePlaybackSnapshot"
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
