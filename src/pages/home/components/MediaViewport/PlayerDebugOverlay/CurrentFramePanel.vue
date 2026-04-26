<script setup lang="ts">
import type { CurrentFrameSection } from "../../../composables/usePlayerDebugOverlay";

defineProps<{
  sections: CurrentFrameSection[];
}>();
</script>

<template>
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-0.5 text-slate-400/95">当前帧</div>
    <div v-if="!sections.length" class="text-slate-100/70">等待首帧与运行态采样...</div>
    <div v-else class="grid grid-cols-1 gap-1.5 xl:grid-cols-3">
      <section
        v-for="section in sections"
        :key="section.id"
        class="rounded-lg border border-slate-400/20 bg-slate-900/25 px-1.5 pb-0.5 pt-1"
      >
        <div class="mb-0.5 text-[10px] uppercase tracking-wide text-slate-400/90">{{ section.title }}</div>
        <div
          v-for="row in section.rows"
          :key="row.key"
          class="mb-0.5 grid grid-cols-[78px_1fr] items-start gap-1.5 opacity-95"
        >
          <span class="lowercase text-slate-400/95">{{ row.label }}</span>
          <span class="wrap-break-word text-slate-100/95">{{ row.value }}</span>
        </div>
      </section>
    </div>
  </div>
</template>
