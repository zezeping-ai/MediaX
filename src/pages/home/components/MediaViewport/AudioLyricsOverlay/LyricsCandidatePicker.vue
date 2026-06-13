<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { onClickOutside } from "@vueuse/core";
import { Icon } from "@iconify/vue";
import type { LyricsCandidateSummary, MediaSnapshot } from "@/modules/media-types";
import { playbackSelectLyricsCandidate } from "@/modules/media-player/playbackCommands";
import { formatLyricsSourceLabel } from "@/modules/lyrics";

const props = defineProps<{
  candidateId: string | null;
  candidates: LyricsCandidateSummary[];
  fetching: boolean;
  transparentOverlay: boolean;
  isDark?: boolean;
  updatePlaybackSnapshot: (snapshot: MediaSnapshot) => void;
}>();

const isDarkTheme = computed(() => props.isDark !== false);

const pickerButtonClass = computed(() => {
  if (props.transparentOverlay) {
    return isDarkTheme.value
      ? "border-white/10 bg-black/35 text-white/88 hover:border-white/16 hover:bg-black/45"
      : "border-black/10 bg-white/72 text-slate-800 hover:border-black/16 hover:bg-white/88";
  }
  if (isDarkTheme.value) {
    return "border-white/14 bg-black/45 text-white/88 hover:border-white/24 hover:bg-black/58";
  }
  return "border-black/10 bg-white/78 text-slate-800 hover:border-black/16 hover:bg-white/90";
});

const pickerBadgeClass = computed(() => (
  isDarkTheme.value
    ? "bg-white/10 text-white/72"
    : "bg-black/6 text-slate-600"
));

const pickerIconClass = computed(() => (
  isDarkTheme.value ? "text-white/45" : "text-slate-400"
));

const pickerMenuClass = computed(() => (
  isDarkTheme.value
    ? "border-white/12 bg-black/82 shadow-[0_16px_40px_rgba(0,0,0,0.45)]"
    : "border-black/10 bg-white/96 shadow-[0_16px_40px_rgba(15,23,42,0.14)]"
));

const pickerItemHoverClass = computed(() => (
  isDarkTheme.value ? "hover:bg-white/8" : "hover:bg-black/5"
));

const pickerItemActiveClass = computed(() => (
  isDarkTheme.value ? "bg-white/10" : "bg-black/6"
));

const pickerIndexClass = computed(() => (
  isDarkTheme.value ? "text-white/42" : "text-slate-400"
));

const pickerTitleClass = computed(() => (
  isDarkTheme.value ? "text-white/90" : "text-slate-800"
));

const pickerPreviewClass = computed(() => (
  isDarkTheme.value ? "text-white/45" : "text-slate-500"
));

const switching = ref(false);
const pickerOpen = ref(false);
const pickerRef = ref<HTMLElement | null>(null);

const dismissPicker = onClickOutside(pickerRef, () => {
  pickerOpen.value = false;
});

onBeforeUnmount(() => {
  dismissPicker();
});

const activeCandidate = computed(() =>
  props.candidates.find((candidate) => candidate.id === props.candidateId) ?? props.candidates[0] ?? null,
);

const currentPickerLabel = computed(() => {
  const candidate = activeCandidate.value;
  if (!candidate) {
    return "选择歌词";
  }
  return shortCandidateLabel(candidate);
});

function shortCandidateLabel(candidate: LyricsCandidateSummary) {
  const preview = candidate.preview.trim();
  if (preview) {
    return truncateText(preview, 28);
  }
  const parts = candidate.label.split("·").map((part) => part.trim()).filter(Boolean);
  const tail = parts[parts.length - 1] ?? candidate.label;
  return truncateText(tail, 28);
}

function candidateMenuTitle(candidate: LyricsCandidateSummary, index: number) {
  const provider = formatLyricsSourceLabel(candidate.provider_id) || `源 ${index + 1}`;
  const preview = candidate.preview.trim();
  if (preview) {
    return `${provider} · ${truncateText(preview, 36)}`;
  }
  return truncateText(candidate.label, 42);
}

function truncateText(value: string, maxChars: number) {
  if (value.length <= maxChars) {
    return value;
  }
  return `${value.slice(0, maxChars - 1)}…`;
}

async function handleCandidateSelect(nextId: string) {
  if (!nextId || nextId === props.candidateId || switching.value) {
    pickerOpen.value = false;
    return;
  }
  switching.value = true;
  try {
    const snapshot = await playbackSelectLyricsCandidate(nextId);
    props.updatePlaybackSnapshot(snapshot);
    pickerOpen.value = false;
  } finally {
    switching.value = false;
  }
}
</script>

<template>
  <div
    ref="pickerRef"
    class="relative"
    data-no-window-drag="true"
  >
    <button
      type="button"
      class="flex max-w-[min(100%,16rem)] items-center gap-2 rounded-lg border px-2.5 py-1.5 text-left text-[12px] transition disabled:opacity-60"
      :class="pickerButtonClass"
      :disabled="fetching || switching"
      @click="pickerOpen = !pickerOpen"
    >
      <span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] tracking-[0.12em]" :class="pickerBadgeClass">
        匹配
      </span>
      <span class="min-w-0 flex-1 truncate tracking-wide">
        {{ currentPickerLabel }}
      </span>
      <Icon
        icon="lucide:chevrons-up-down"
        width="14"
        height="14"
        class="shrink-0"
        :class="pickerIconClass"
      />
    </button>

    <div
      v-if="pickerOpen"
      class="absolute right-0 top-[calc(100%+0.35rem)] z-30 max-h-52 w-[min(100vw-2.5rem,20rem)] overflow-y-auto rounded-xl border p-1 backdrop-blur-md scrollbar-none"
      :class="pickerMenuClass"
    >
      <button
        v-for="(candidate, index) in candidates"
        :key="candidate.id"
        type="button"
        class="flex w-full items-start gap-2 rounded-lg px-2.5 py-2 text-left transition"
        :class="[
          pickerItemHoverClass,
          candidate.id === candidateId ? pickerItemActiveClass : '',
        ]"
        @click="void handleCandidateSelect(candidate.id)"
      >
        <span class="mt-0.5 shrink-0 text-[10px] tracking-[0.14em]" :class="pickerIndexClass">
          {{ index + 1 }}
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-[12px] tracking-wide" :class="pickerTitleClass">
            {{ candidateMenuTitle(candidate, index) }}
          </span>
          <span
            v-if="candidate.preview.trim()"
            class="mt-0.5 block truncate text-[11px] tracking-wide"
            :class="pickerPreviewClass"
          >
            {{ truncateText(candidate.preview.trim(), 40) }}
          </span>
        </span>
      </button>
    </div>
  </div>
</template>
