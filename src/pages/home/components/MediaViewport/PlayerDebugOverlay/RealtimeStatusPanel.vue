<script setup lang="ts">
import type { DebugGroup } from "../../../composables/usePlayerDebugOverlay";

defineProps<{
  groups: DebugGroup[];
}>();
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-0.5 text-slate-400/95">实时状态</div>
    <div v-for="group in groups" :key="group.id" class="mb-1 rounded-lg border border-slate-400/20 bg-slate-900/25 px-1.5 pb-0.5 pt-1">
      <div class="mb-0.5 text-[10px] uppercase tracking-wide text-slate-400/90">{{ group.title }}</div>
      <div v-for="row in group.rows" :key="row.key" class="mb-0.5 grid grid-cols-[70px_1fr] items-start gap-1.5 opacity-95">
        <span class="lowercase text-slate-400/95">{{ row.label }}</span>
        <span class="wrap-break-word text-slate-100/95">{{ row.value }}</span>
      </div>
    </div>
  </div>
</template>
