<script setup lang="ts">
import { computed, ref, toRef, watch } from "vue";
import { type PlaybackState } from "@/modules/media-types";
import { usePreferences } from "@/modules/preferences";
import { usePlayerDebugOverlay } from "../composables/usePlayerDebugOverlay";

const props = defineProps<{
  source: string;
  loading: boolean;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
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

const { decodeBanner, debugGroups } = usePlayerDebugOverlay(
  toRef(props, "playback"),
  toRef(props, "debugSnapshot"),
);

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
      v-if="canShowDebugOverlay"
      class="debug-overlay absolute left-4 top-4 z-5 flex h-[min(58vh,430px)] min-h-[250px] w-[min(620px,calc(100vw-32px))] min-w-[380px] max-h-[calc(100vh-24px)] max-w-[calc(100vw-24px)] resize flex-col gap-1.5 overflow-hidden rounded-xl border border-white/16 bg-[linear-gradient(180deg,rgba(11,16,23,0.86)_0%,rgba(9,13,20,0.78)_100%)] px-2 py-2 pl-2.5 font-mono text-[11px] leading-4 text-slate-100/95 shadow-[0_10px_30px_rgba(0,0,0,0.28)] backdrop-blur-[14px] max-[720px]:min-w-[320px]"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-1.5">
          <div class="font-bold tracking-[0.2px]">解析 Debug</div>
          <span class="inline-flex h-4 items-center justify-center rounded-full border border-blue-500/45 bg-blue-500/20 px-1.5 text-[10px] text-emerald-100">LIVE</span>
        </div>
        <a-button class="debug-close-btn" size="mini" type="text" @click="closeCurrentDebugOverlay">
          关闭
        </a-button>
      </div>
      <div v-if="decodeBanner" class="rounded-[10px] border border-white/12 bg-slate-900/55 px-2 py-1.5">
        <div class="flex items-center gap-2">
          <span
            class="inline-flex min-w-[42px] shrink-0 items-center justify-center rounded-[7px] border px-2 py-1 text-xs font-extrabold leading-[1.2] tracking-[0.4px] shadow-[0_2px_10px_rgba(0,0,0,0.2)]"
            :class="
              decodeBanner.isHardware
                ? 'border-emerald-400/55 bg-[linear-gradient(135deg,rgba(16,185,129,0.35)_0%,rgba(5,150,105,0.45)_100%)] text-emerald-50'
                : 'border-amber-300/50 bg-[linear-gradient(135deg,rgba(245,158,11,0.32)_0%,rgba(217,119,6,0.42)_100%)] text-orange-50'
            "
          >
            {{ decodeBanner.isHardware ? "硬解" : "软解" }}
          </span>
          <div class="flex min-w-0 flex-wrap gap-x-2.5 gap-y-0.5">
            <span class="whitespace-nowrap text-slate-100/95">backend: {{ decodeBanner.backend }}</span>
            <span class="whitespace-nowrap text-slate-100/95">mode: {{ decodeBanner.modeLabel }} ({{ decodeBanner.mode }})</span>
            <span v-if="decodeBanner.error" class="whitespace-nowrap text-rose-200">err: {{ decodeBanner.error }}</span>
          </div>
        </div>
      </div>
      <div class="debug-log-wrap flex min-h-0 flex-1 flex-col overflow-auto rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
        <div class="mb-0.5 text-slate-400/95">实时状态</div>
        <div v-for="group in debugGroups" :key="group.id" class="mb-1 rounded-lg border border-slate-400/20 bg-slate-900/25 px-1.5 pb-0.5 pt-1">
          <div v-for="row in group.rows" :key="row.key" class="mb-0.5 grid grid-cols-[70px_1fr] items-start gap-1.5 opacity-95">
            <span class="lowercase text-slate-400/95">{{ row.label }}</span>
            <span class="wrap-break-word text-slate-100/95">{{ row.value }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.debug-log-wrap {
  scrollbar-width: thin;
  scrollbar-color: rgba(148, 163, 184, 0.42) transparent;
  color-scheme: dark;
}

.debug-log-wrap::-webkit-scrollbar {
  width: 6px;
  height: 6px;
  background: transparent;
}

.debug-log-wrap::-webkit-scrollbar-track {
  background: transparent;
}

.debug-log-wrap::-webkit-scrollbar-corner {
  background: transparent;
}

.debug-log-wrap::-webkit-scrollbar-thumb {
  border-radius: 999px;
  border: 1px solid transparent;
  background-clip: padding-box;
  background: rgba(148, 163, 184, 0.4);
  transition: background-color 180ms ease;
}

.debug-log-wrap:hover::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.65);
}

.debug-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.86);
  padding: 0 8px;
  height: 22px;
  min-width: 46px;
  line-height: 1;
  border-radius: 6px;
}

.debug-close-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.12);
}
</style>
