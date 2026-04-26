<script setup lang="ts">
import type { ProcessStage } from "../../../composables/usePlayerDebugOverlay";

defineProps<{
  stages: ProcessStage[];
}>();

function formatTime(ms: number | null) {
  if (!ms) {
    return "--:--:--.---";
  }
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  const mss = String(d.getMilliseconds()).padStart(3, "0");
  return `${hh}:${mm}:${ss}.${mss}`;
}

function statusTone(status: ProcessStage["status"]) {
  switch (status) {
    case "completed":
      return "border-emerald-400/30 bg-emerald-400/10 text-emerald-50";
    case "active":
      return "border-cyan-300/30 bg-cyan-400/10 text-cyan-50";
    case "error":
      return "border-rose-400/30 bg-rose-400/10 text-rose-50";
    default:
      return "border-white/10 bg-white/[0.03] text-slate-300/80";
  }
}

function formatDuration(ms: number | null) {
  if (ms === null || !Number.isFinite(ms)) {
    return "--";
  }
  if (ms < 1000) {
    return `${Math.round(ms)}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-1 text-slate-400/95">解析阶段</div>
    <div class="space-y-1">
      <div
        v-for="stage in stages"
        :key="stage.id"
        :class="['rounded-md border px-2 py-1.5', statusTone(stage.status)]"
      >
        <div class="flex items-center justify-between gap-2 text-[10px]">
          <div class="flex items-center gap-1.5">
            <span class="inline-flex h-2 w-2 rounded-full bg-current opacity-85" />
            <span class="font-semibold">{{ stage.label }}</span>
          </div>
          <div class="flex items-center gap-2 opacity-70">
            <span>{{ formatTime(stage.atMs) }}</span>
            <span v-if="stage.sinceStartMs !== null">T+{{ formatDuration(stage.sinceStartMs) }}</span>
            <span v-if="stage.sincePrevMs !== null">Δ{{ formatDuration(stage.sincePrevMs) }}</span>
          </div>
        </div>
        <div class="mt-0.5 wrap-break-word text-[11px] leading-4 opacity-95">{{ stage.message }}</div>
      </div>
    </div>
  </div>
</template>
