<script setup lang="ts">
import { ref } from "vue";
import { useIntervalFn } from "@vueuse/core";
import { formatPlaybackClock } from "@/modules/format-playback-clock";

const AUTO_DISMISS_SECONDS = 15;

defineProps<{
  positionSeconds: number;
}>();

const emit = defineEmits<{
  accept: [];
  dismiss: [];
}>();

const remainingSeconds = ref(AUTO_DISMISS_SECONDS);

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
  <span class="inline-flex min-w-0 items-center gap-1.5 truncate text-[11px] text-white/62">
    <span class="truncate">继续至 {{ formatPlaybackClock(positionSeconds) }}</span>
    <button
      type="button"
      class="shrink-0 cursor-pointer text-white/88 transition hover:text-white hover:underline"
      @click="accept"
    >
      跳转
    </button>
    <span class="text-white/28">·</span>
    <button
      type="button"
      class="shrink-0 cursor-pointer transition hover:text-white/78 hover:underline"
      @click="dismiss"
    >
      忽略 {{ remainingSeconds }}
    </button>
  </span>
</template>
