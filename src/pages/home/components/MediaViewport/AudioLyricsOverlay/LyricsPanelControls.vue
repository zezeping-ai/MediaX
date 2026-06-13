<script setup lang="ts">
import { computed, toRef } from "vue";
import { formatLyricsOffsetLabel } from "@/modules/lyrics";
import { Icon } from "@iconify/vue";
import { useOverlaySurfaceTheme } from "./lyrics/useOverlaySurfaceTheme";

const props = defineProps<{
  lyricsVisible: boolean;
  offsetSeconds: number;
  offsetStepSeconds: number;
  hasSyncedLyrics: boolean;
  dragging: boolean;
  isDark?: boolean;
}>();

const emit = defineEmits<{
  "toggle-visible": [];
  "reset-offset": [];
  "adjust-offset": [deltaSeconds: number];
}>();

const { isDarkTheme, pick } = useOverlaySurfaceTheme({
  isDark: toRef(props, "isDark"),
});

const controlButtonClass = pick(
  "border-black/10 bg-white/78 text-slate-800 hover:border-black/16 hover:bg-white/90",
  "border-white/14 bg-black/45 text-white/88 hover:border-white/24 hover:bg-black/58",
);

const controlButtonActiveClass = computed(() => (
  isDarkTheme.value
    ? "border-white/24 bg-white/12 text-white"
    : "border-black/16 bg-black/6 text-slate-900"
));

const offsetGroupClass = pick(
  "border-black/10 bg-white/78 text-slate-600",
  "border-white/14 bg-black/45 text-white/72",
);

const offsetStepperButtonClass = computed(() => (
  isDarkTheme.value
    ? "text-white/78 hover:bg-white/10 disabled:text-white/28"
    : "text-slate-700 hover:bg-black/5 disabled:text-slate-300"
));

const offsetLabel = computed(() => formatLyricsOffsetLabel(props.offsetSeconds));
const stepLabel = computed(() => `${props.offsetStepSeconds}s`);
</script>

<template>
  <div class="pointer-events-auto flex shrink-0 items-center gap-1.5">
    <button
      type="button"
      class="flex h-7 w-7 items-center justify-center rounded-lg border transition"
      :class="[
        controlButtonClass,
        !lyricsVisible ? controlButtonActiveClass : '',
      ]"
      :title="lyricsVisible ? '隐藏当前歌曲歌词' : '显示当前歌曲歌词'"
      data-no-window-drag="true"
      @click="emit('toggle-visible')"
    >
      <Icon
        :icon="lyricsVisible ? 'lucide:eye' : 'lucide:eye-off'"
        width="14"
        height="14"
        aria-hidden="true"
      />
    </button>

    <div
      v-if="hasSyncedLyrics"
      class="flex items-center overflow-hidden rounded-lg border"
      :class="offsetGroupClass"
    >
      <button
        type="button"
        class="flex h-7 w-6 items-center justify-center transition disabled:cursor-not-allowed"
        :class="offsetStepperButtonClass"
        :disabled="dragging"
        :title="`歌词提前 ${stepLabel}`"
        data-no-window-drag="true"
        @click="emit('adjust-offset', -offsetStepSeconds)"
      >
        <Icon icon="lucide:minus" width="12" height="12" aria-hidden="true" />
      </button>

      <button
        type="button"
        class="min-w-11 border-x px-1.5 py-1 text-[10px] font-medium tabular-nums transition hover:opacity-80 disabled:cursor-not-allowed"
        :class="[
          isDarkTheme ? 'border-white/10' : 'border-black/8',
          dragging ? 'opacity-70' : '',
        ]"
        :disabled="dragging"
        :title="dragging ? '拖动歌词校准中' : '歌词时间偏移，点击重置'"
        data-no-window-drag="true"
        @click="emit('reset-offset')"
      >
        {{ dragging ? "校准中…" : offsetLabel }}
      </button>

      <button
        type="button"
        class="flex h-7 w-6 items-center justify-center transition disabled:cursor-not-allowed"
        :class="offsetStepperButtonClass"
        :disabled="dragging"
        :title="`歌词延后 ${stepLabel}`"
        data-no-window-drag="true"
        @click="emit('adjust-offset', offsetStepSeconds)"
      >
        <Icon icon="lucide:plus" width="12" height="12" aria-hidden="true" />
      </button>
    </div>
  </div>
</template>
