<script setup lang="ts">
import { toRef } from "vue";
import { useOverlaySurfaceTheme } from "./lyrics/useOverlaySurfaceTheme";

const props = defineProps<{
  message: string;
  actionLabel?: string;
  isDark?: boolean;
}>();

const emit = defineEmits<{
  action: [];
}>();

const { pick } = useOverlaySurfaceTheme({
  isDark: toRef(props, "isDark"),
});

const emptyClass = pick("text-slate-600", "text-white/68");
const actionClass = pick(
  "border-black/10 bg-black/4 text-slate-700 hover:bg-black/6",
  "border-white/14 bg-white/8 text-white/82 hover:bg-white/12",
);
</script>

<template>
  <div
    class="flex min-h-32 flex-1 flex-col items-center justify-center gap-2 text-sm tracking-wide"
    :class="emptyClass"
  >
    <span>{{ message }}</span>
    <button
      v-if="actionLabel"
      type="button"
      class="pointer-events-auto rounded-lg border px-3 py-1.5 text-[12px] transition"
      :class="actionClass"
      data-no-window-drag="true"
      @click="emit('action')"
    >
      {{ actionLabel }}
    </button>
  </div>
</template>
