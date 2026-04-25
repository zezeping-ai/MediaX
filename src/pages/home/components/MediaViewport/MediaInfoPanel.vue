<script setup lang="ts">
defineProps<{
  rows: Array<{ key: string; label: string; value: string }>;
}>();
</script>

<template>
  <div v-if="rows.length" class="rounded-[10px] border border-white/12 bg-slate-900/40 px-2 py-1.5">
    <div class="mb-0.5 text-[10px] text-slate-400/95">媒体信息</div>
    <div v-for="row in rows" :key="row.key" class="mb-0.5 grid grid-cols-[70px_1fr] items-start gap-1.5 opacity-95">
      <span class="lowercase text-slate-400/95">{{ row.label }}</span>
      <span class="wrap-break-word text-slate-100/95">{{ row.value }}</span>
    </div>
  </div>
</template>

