<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from "vue";
import {
  type MediaAudioMeterPayload,
  type MediaLyricLine,
  type MediaTelemetryPayload,
  type PlaybackChannelRouting,
  type PlaybackState,
} from "@/modules/media-types";
import {
  startMainWindowDragging,
  toggleMainWindowFullscreen,
} from "@/modules/media-player/windowCommands";
import { usePreferences } from "@/modules/preferences";
import AudioLyricsOverlay from "./AudioLyricsOverlay";
import TransferStatusOverlay from "./TransferStatusOverlay.vue";
import LoadingProcessOverlay from "./PlayerDebugOverlay/LoadingProcessOverlay.vue";

const PlayerDebugOverlay = defineAsyncComponent({
  loader: () => import("./PlayerDebugOverlay"),
  delay: 120,
});

const props = defineProps<{
  source: string;
  pendingSource: string;
  controlsVisible: boolean;
  loading: boolean;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
  debugTimeline: Array<{ stage: string; message: string; at_ms: number }>;
  debugStageSnapshot: Record<string, { message: string; at_ms: number }>;
  firstFrameAtMs: number | null;
  latestTelemetry: MediaTelemetryPayload | null;
  latestAudioMeter: MediaAudioMeterPayload | null;
  telemetryHistory: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
  mediaInfoSnapshot: Record<string, string>;
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

const { playerParseDebugEnabled } = usePreferences();
const debugOverlayOpen = ref(true);

const hasVideoPresentationSignals = computed(() =>
  Boolean(
    props.playback?.media_kind === "video"
    || props.debugSnapshot.video_frame_format
    || props.debugSnapshot.video_fps
    || props.debugSnapshot.video_pipeline,
  ),
);
const hasAudioPresentationSignals = computed(() =>
  Boolean(
    props.playback?.media_kind === "audio"
    || props.metadataMediaKind === "audio"
    || (
      !hasVideoPresentationSignals.value
      && (
        props.metadataHasCoverArt
        || Boolean(props.latestAudioMeter?.channels)
        || Boolean(props.debugSnapshot.audio_pipeline_ready)
      )
    ),
  ),
);
const effectiveMediaKind = computed(() => {
  if (hasAudioPresentationSignals.value) {
    return "audio";
  }
  return "video";
});
const overlaySource = computed(() => props.pendingSource || props.source);
const hasPresentedFirstFrame = computed(() =>
  Boolean(
    effectiveMediaKind.value === "audio"
    || props.debugSnapshot.audio_pipeline_ready
    || hasVideoPresentationSignals.value,
  ),
);
const isWaitingForFirstFrame = computed(() => (
  Boolean(overlaySource.value)
  && !hasPresentedFirstFrame.value
  && props.playback?.status !== "stopped"
));
const shouldShowParseOverlay = computed(() => (
  props.controlsVisible
  && playerParseDebugEnabled.value
  && debugOverlayOpen.value
  && Boolean(overlaySource.value)
));
const shouldShowLoadingProcessOverlay = computed(() => (
  shouldShowParseOverlay.value
  && isWaitingForFirstFrame.value
));
const shouldShowDebugOverlay = computed(() => (
  shouldShowParseOverlay.value
  && Boolean(props.source)
  && Boolean(props.firstFrameAtMs)
));
watch(
  () => props.source,
  (nextSource) => {
    if (!nextSource) {
      emit("ended");
      debugOverlayOpen.value = true;
      return;
    }
    // New source: reopen overlay so user can see parse stages.
    debugOverlayOpen.value = true;
  },
);

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
    <LoadingProcessOverlay
      v-if="shouldShowLoadingProcessOverlay"
      :source="overlaySource"
      :timeline="debugTimeline"
    />
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
    <PlayerDebugOverlay
      v-if="shouldShowDebugOverlay"
      :source="source"
      :playback="playback"
      :debug-snapshot="debugSnapshot"
      :debug-timeline="debugTimeline"
      :debug-stage-snapshot="debugStageSnapshot"
      :latest-telemetry="latestTelemetry"
      :telemetry-history="telemetryHistory"
      :media-info-snapshot="mediaInfoSnapshot"
      @close="debugOverlayOpen = false"
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
