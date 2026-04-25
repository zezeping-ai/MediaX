<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { type PlaybackState } from "@/modules/media-types";
import { usePreferences } from "@/modules/preferences";
import TransferStatusOverlay from "./TransferStatusOverlay.vue";
import PlayerDebugOverlay from "./PlayerDebugOverlay.vue";

const props = defineProps<{
  source: string;
  loading: boolean;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
  debugTimeline: Array<{ stage: string; message: string; at_ms: number }>;
  mediaInfoSnapshot: Record<string, string>;
  networkReadBytesPerSecond: number | null;
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
  () => Boolean(props.source) && playerParseDebugEnabled.value && debugOverlayOpen.value,
);

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
    <a-spin v-if="loading" class="absolute" />
    <PlayerDebugOverlay
      v-if="shouldShowDebugOverlay"
      :source="source"
      :playback="playback"
      :debug-snapshot="debugSnapshot"
      :debug-timeline="debugTimeline"
      :media-info-snapshot="mediaInfoSnapshot"
      @close="debugOverlayOpen = false"
    />
    <TransferStatusOverlay
      :source="source"
      :network-read-bytes-per-second="networkReadBytesPerSecond"
      :cache-recording="cacheRecording"
      :cache-output-path="cacheOutputPath"
      :cache-output-size-bytes="cacheOutputSizeBytes"
      :cache-write-speed-bytes-per-second="cacheWriteSpeedBytesPerSecond"
    />
  </section>
</template>

<style scoped>
</style>
