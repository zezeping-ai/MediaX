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
}>();

const emit = defineEmits<{
  ended: [];
  "quick-open-local": [];
  "quick-open-url": [];
}>();

const { playerParseDebugEnabled } = usePreferences();
const debugDismissedSource = ref("");

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
