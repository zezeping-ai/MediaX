<script setup lang="ts">
defineProps<{
  resourceSummary: string;
  renderSummary: string;
}>();
</script>

<template>
  <div v-if="resourceSummary || renderSummary" class="mt-1 rounded-[8px] border border-white/10 bg-slate-950/30 px-2 py-1">
    <div v-if="resourceSummary" class="grid grid-cols-[70px_1fr] items-start gap-1.5 opacity-95">
      <span class="lowercase text-[10px] text-slate-400/95">资源占用</span>
      <span class="wrap-break-word text-slate-100/95">{{ resourceSummary }}</span>
    </div>
    <div v-if="renderSummary" class="mt-1 grid grid-cols-[70px_1fr] items-start gap-1.5 opacity-95">
      <span class="lowercase text-[10px] text-slate-400/95">渲染估计</span>
      <span class="wrap-break-word text-slate-100/95">{{ renderSummary }}</span>
    </div>
  </div>
</template>

