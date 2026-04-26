<script setup lang="ts">
defineProps<{
  backend: string;
  mode: string;
  modeLabel: string;
  error: string | null | undefined;
}>();
</script>

<template>
  <div class="rounded-[10px] border border-white/12 bg-slate-900/55 px-2 py-1.5">
    <div class="flex items-center gap-2">
      <span class="text-slate-400/90">backend:</span>
      <span class="text-slate-100/95">{{ backend }}</span>
      <span class="text-slate-400/90">mode:</span>
      <span class="text-slate-100/95">{{ modeLabel }} ({{ mode }})</span>
      <span v-if="error" class="text-rose-200">err: {{ error }}</span>
    </div>
  </div>
</template>
