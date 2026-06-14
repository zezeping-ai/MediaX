<script setup lang="ts">
import { formatLyricsOptionMeta, formatLyricsOptionTitle } from "./labels";
import type { LyricsSelectOption } from "./types";

defineProps<{
  option: LyricsSelectOption;
  index: number;
  active: boolean;
  titleClass: string;
  metaClass: string;
  indexClass: string;
  itemHoverClass: string;
  itemActiveClass: string;
}>();

const emit = defineEmits<{
  select: [string];
}>();
</script>

<template>
  <button
    type="button"
    class="flex w-full items-start gap-2 rounded-lg px-2.5 py-2 text-left transition"
    :class="[itemHoverClass, active ? itemActiveClass : '']"
    @click="emit('select', option.id)"
  >
    <span class="mt-0.5 shrink-0 text-[10px] tracking-[0.14em]" :class="indexClass">
      {{ index + 1 }}
    </span>
    <span class="min-w-0 flex-1">
      <span class="block truncate text-[12px] tracking-wide" :class="titleClass">
        {{ formatLyricsOptionTitle(option) }}
      </span>
      <span class="mt-0.5 block truncate text-[11px] tracking-wide" :class="metaClass">
        {{ formatLyricsOptionMeta(option) }}
      </span>
    </span>
  </button>
</template>
