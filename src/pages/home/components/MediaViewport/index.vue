<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { type PlaybackState } from "@/modules/media-types";
import { usePreferences } from "@/modules/preferences";
import TransferStatusOverlay from "./TransferStatusOverlay.vue";
import LoadingProcessOverlay from "./PlayerDebugOverlay/LoadingProcessOverlay.vue";
import PlayerDebugOverlay from "./PlayerDebugOverlay/index.vue";

const props = defineProps<{
  source: string;
  pendingSource: string;
  loading: boolean;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
  debugTimeline: Array<{ stage: string; message: string; at_ms: number }>;
  firstFrameAtMs: number | null;
  mediaInfoSnapshot: Record<string, string>;
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

const shouldShowDebugOverlay = computed(
  () =>
    Boolean(props.source)
    && playerParseDebugEnabled.value
    && debugOverlayOpen.value
    && Boolean(props.firstFrameAtMs),
);
const overlaySource = computed(() => props.pendingSource || props.source);
const hasPresentedFirstFrame = computed(() =>
  Boolean(
    props.debugSnapshot.video_frame_format
    || props.debugSnapshot.video_fps
    || props.debugSnapshot.video_pipeline,
  ),
);
const isWaitingForFirstFrame = computed(
  () =>
    Boolean(overlaySource.value)
    && !hasPresentedFirstFrame.value
    && props.playback?.status !== "stopped",
);
const shouldShowLoadingProcessOverlay = computed(
  () =>
    playerParseDebugEnabled.value
    && Boolean(overlaySource.value)
    && isWaitingForFirstFrame.value,
);
const activeDebugTimeline = computed(() => {
  if (!props.firstFrameAtMs) {
    return [];
  }
  return props.debugTimeline.filter((item) => item.at_ms >= props.firstFrameAtMs!);
});
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
</script>

<template>
  <section class="relative flex h-full items-center justify-center overflow-hidden bg-transparent">
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
    <PlayerDebugOverlay
      v-if="shouldShowDebugOverlay"
      :source="source"
      :playback="playback"
      :debug-snapshot="debugSnapshot"
      :debug-timeline="activeDebugTimeline"
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
