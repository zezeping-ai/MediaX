<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { type PlaybackState } from "@/modules/media-types";
import { usePreferences } from "@/modules/preferences";
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
const debugDismissedSource = ref("");

function isUrlSource(value: string) {
  const v = (value || "").trim().toLowerCase();
  return (
    v.startsWith("http://") ||
    v.startsWith("https://") ||
    v.startsWith("rtsp://") ||
    v.startsWith("rtmp://") ||
    v.startsWith("mms://")
  );
}

function formatBytes(value: number | null) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unitIdx = 0;
  while (size >= 1024 && unitIdx < units.length - 1) {
    size /= 1024;
    unitIdx += 1;
  }
  const precision = size >= 100 || unitIdx === 0 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(precision)} ${units[unitIdx]}`;
}

const canShowDownloadSpeed = computed(
  () => Boolean(props.source) && isUrlSource(props.source) && props.networkReadBytesPerSecond !== null,
);
const downloadSpeedText = computed(() => `${formatBytes(props.networkReadBytesPerSecond)}/s`);
const canShowCacheRecording = computed(() => Boolean(props.cacheRecording && props.cacheOutputPath));
const cachePathShort = computed(() => {
  const full = props.cacheOutputPath || "";
  if (full.length <= 42) return full;
  return `${full.slice(0, 14)}…${full.slice(-24)}`;
});

const canShowDebugOverlay = computed(
  () =>
    playerParseDebugEnabled.value &&
    Boolean(props.source) &&
    debugDismissedSource.value !== props.source,
);

function closeCurrentDebugOverlay() {
  if (!props.source) {
    return;
  }
  debugDismissedSource.value = props.source;
}

watch(
  () => props.source,
  (nextSource) => {
    if (!nextSource) {
      emit("ended");
      debugDismissedSource.value = "";
    }
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

    <div
      v-if="canShowDownloadSpeed || canShowCacheRecording"
      class="pointer-events-none absolute right-4 top-4 z-20 flex max-w-[min(360px,calc(100vw-32px))] flex-col gap-2"
    >
      <div
        v-if="canShowDownloadSpeed"
        class="rounded-xl border border-white/12 bg-slate-950/30 px-3 py-2 text-[12px] leading-4 text-white/90 shadow-[0_10px_30px_rgba(0,0,0,0.25)] backdrop-blur-md"
      >
        <div class="flex items-center justify-between gap-3">
          <span class="text-white/65">下载速度</span>
          <span class="tabular-nums text-white/95">{{ downloadSpeedText }}</span>
        </div>
      </div>

      <div
        v-if="canShowCacheRecording"
        class="rounded-xl border border-white/12 bg-slate-950/30 px-3 py-2 text-[12px] leading-4 text-white/90 shadow-[0_10px_30px_rgba(0,0,0,0.25)] backdrop-blur-md"
      >
        <div class="mb-1 flex items-center justify-between gap-3">
          <span class="text-white/65">录制写入</span>
          <span class="tabular-nums text-white/95">
            {{ formatBytes(cacheOutputSizeBytes) }}
            <span class="text-white/50">·</span>
            {{ formatBytes(cacheWriteSpeedBytesPerSecond) }}/s
          </span>
        </div>
        <div class="text-[11px] text-white/55" :title="cacheOutputPath">{{ cachePathShort }}</div>
      </div>
    </div>

    <PlayerDebugOverlay
      v-if="canShowDebugOverlay"
      :source="source"
      :playback="playback"
      :debug-snapshot="debugSnapshot"
      :debug-timeline="debugTimeline"
      :media-info-snapshot="mediaInfoSnapshot"
      @close="closeCurrentDebugOverlay"
    />
  </section>
</template>

<style scoped>
</style>
