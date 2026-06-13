<script setup lang="ts">
import { ref } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { formatPlaybackClock } from "@/modules/format-playback-clock";
import { usePlayerChromeTheme } from "@/pages/home/composables/usePlayerChromeTheme";

const AUTO_DISMISS_SECONDS = 15;

defineProps<{
  positionSeconds: number;
}>();

const emit = defineEmits<{
  accept: [];
  dismiss: [];
}>();

const remainingSeconds = ref(AUTO_DISMISS_SECONDS);
const { isDark } = usePlayerChromeTheme();

const { pause: pauseCountdown } = useIntervalFn(() => {
  remainingSeconds.value -= 1;
  if (remainingSeconds.value <= 0) {
    dismiss();
  }
}, 1000);

function accept() {
  pauseCountdown();
  emit("accept");
}

function dismiss() {
  pauseCountdown();
  emit("dismiss");
}
</script>

<template>
  <span class="inline-flex min-w-0 items-center gap-1.5 truncate text-[11px]" :class="isDark ? 'text-white/62' : 'text-slate-600'">
    <span class="truncate">继续至 {{ formatPlaybackClock(positionSeconds) }}</span>
    <button
      type="button"
      class="shrink-0 cursor-pointer transition hover:underline"
      :class="isDark ? 'text-white/88 hover:text-white' : 'text-slate-800 hover:text-slate-950'"
      @click="accept"
    >
      跳转
    </button>
    <span :class="isDark ? 'text-white/28' : 'text-slate-400'">·</span>
    <span class="shrink-0 tabular-nums" :class="isDark ? 'text-white/42' : 'text-slate-500'">
      {{ remainingSeconds }}s
    </span>
  </span>
</template>
