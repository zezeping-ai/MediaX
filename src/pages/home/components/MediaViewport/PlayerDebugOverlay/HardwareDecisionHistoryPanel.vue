<script setup lang="ts">
import type { HardwareDecisionEvent } from "../../../composables/usePlayerDebugOverlay";

defineProps<{
  events: HardwareDecisionEvent[];
}>();

function formatTime(ms: number) {
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  const mss = String(d.getMilliseconds()).padStart(3, "0");
  return `${hh}:${mm}:${ss}.${mss}`;
}

function toneClass(tone: HardwareDecisionEvent["tone"]) {
  switch (tone) {
    case "good":
      return "border-emerald-400/25 bg-emerald-500/8";
    case "warn":
      return "border-amber-300/25 bg-amber-500/8";
    case "error":
      return "border-rose-400/25 bg-rose-500/8";
    default:
      return "border-slate-400/16 bg-slate-900/20";
  }
}
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-0.5 text-slate-400/95">硬解尝试历史</div>
    <div v-if="!events.length" class="text-slate-100/70">等待硬解决策事件...</div>
    <div v-else class="space-y-1">
      <div
        v-for="(item, idx) in events.slice().reverse()"
        :key="`${item.atMs}-${idx}`"
        :class="['rounded-md border px-1.5 py-1', toneClass(item.tone)]"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="text-[10px] text-slate-400/95">{{ item.label }}</span>
          <span class="text-[10px] text-slate-500/90">{{ formatTime(item.atMs) }}</span>
        </div>
        <div class="mt-0.5 wrap-break-word text-slate-100/95">{{ item.message }}</div>
      </div>
    </div>
  </div>
</template>
