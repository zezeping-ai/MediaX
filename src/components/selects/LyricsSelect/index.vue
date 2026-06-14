<script setup lang="ts">
import { computed, watch } from "vue";
import { Icon } from "@iconify/vue";
import LyricsSelectOptionRow from "./LyricsSelectOptionRow.vue";
import { formatLyricsTriggerLabel } from "./labels";
import { useLyricsSelectCandidates } from "./useLyricsSelectCandidates";
import { useLyricsSelectDropdown } from "./useLyricsSelectDropdown";
import { useLyricsSelectSearch } from "./useLyricsSelectSearch";
import { useLyricsSelectTheme } from "./useLyricsSelectTheme";
import type { LyricsCandidateSummary, MediaSnapshot } from "@/modules/media-types";

const props = withDefaults(defineProps<{
  mode: "search" | "candidates";
  disabled?: boolean;
  searchDisabled?: boolean;
  compact?: boolean;
  overlay?: boolean;
  isDark?: boolean;
  transparentOverlay?: boolean;
  fetching?: boolean;
  title?: string;
  artist?: string;
  album?: string;
  durationSeconds?: number;
  lyricsSource?: string | null;
  candidates?: LyricsCandidateSummary[];
  selectedId?: string | null;
  updatePlaybackSnapshot?: (snapshot: MediaSnapshot) => void;
}>(), {
  disabled: false,
  searchDisabled: false,
  compact: false,
  overlay: false,
  transparentOverlay: false,
  fetching: false,
  title: "",
  artist: "",
  album: "",
  durationSeconds: 0,
  lyricsSource: null,
  candidates: () => [],
  selectedId: null,
});

const lyricsLrc = defineModel<string>("lyricsLrc", { default: "" });

const {
  close,
  open,
  rootRef,
  toggleOpen,
} = useLyricsSelectDropdown();

const theme = useLyricsSelectTheme({
  overlay: computed(() => props.overlay),
  isDark: computed(() => props.isDark),
  transparentOverlay: computed(() => props.transparentOverlay),
});

const searchState = props.mode === "search"
  ? useLyricsSelectSearch({
      query: () => ({
        title: props.title ?? "",
        artist: props.artist ?? "",
        album: props.album ?? "",
        durationSeconds: props.durationSeconds ?? 0,
      }),
      lyricsLrc,
      localSource: () => props.lyricsSource ?? null,
    })
  : null;

const candidatesState = props.mode === "candidates" && props.updatePlaybackSnapshot
  ? useLyricsSelectCandidates({
      candidates: () => props.candidates ?? [],
      selectedId: () => props.selectedId ?? null,
      updatePlaybackSnapshot: props.updatePlaybackSnapshot,
    })
  : null;

const options = computed(() => (
  props.mode === "search"
    ? searchState?.options.value ?? []
    : candidatesState?.options.value ?? []
));

const hasOptions = computed(() => options.value.length > 0);
const selectedOption = computed(() => (
  props.mode === "search"
    ? searchState?.selectedOption.value ?? null
    : candidatesState?.selectedOption.value ?? null
));

const triggerLabel = computed(() => {
  if (props.disabled || props.searchDisabled) {
    return props.mode === "search" ? "请先开启嵌入歌词" : "选择歌词";
  }
  if (!selectedOption.value) {
    return props.mode === "search" ? "检索后选择歌词" : "选择歌词";
  }
  if (props.compact) {
    return formatLyricsTriggerLabel(selectedOption.value);
  }
  const { title, artist } = selectedOption.value;
  return `${title} · ${artist}`;
});

const isBusy = computed(() => (
  props.disabled
  || props.fetching
  || Boolean(searchState?.searching.value)
  || Boolean(candidatesState?.switching.value)
));

const canOpen = computed(() => hasOptions.value && !isBusy.value);

watch(
  () => props.disabled || props.searchDisabled,
  (locked) => {
    if (locked) {
      close();
    }
  },
);

function handleSelect(optionId: string) {
  if (props.mode === "search") {
    searchState?.selectOption(optionId);
  } else {
    void candidatesState?.selectOption(optionId);
  }
  close();
}

function handleSearch() {
  void searchState?.search();
}

function reset() {
  searchState?.reset();
}

function resolveSelectedLyrics() {
  return searchState?.resolveSelectedLyrics() ?? lyricsLrc.value.trim();
}

defineExpose({ reset, resolveSelectedLyrics });
</script>

<template>
  <div class="flex items-center gap-2">
    <div
      ref="rootRef"
      class="relative min-w-0"
      :class="compact ? 'max-w-[min(100%,16rem)]' : 'flex-1'"
      data-no-window-drag="true"
    >
      <button
        type="button"
        class="flex w-full items-center gap-2 rounded-lg border px-2.5 py-1.5 text-left text-[12px] transition disabled:cursor-not-allowed disabled:opacity-60"
        :class="theme.triggerClass"
        :disabled="isBusy || (mode === 'search' && !hasOptions)"
        @click="canOpen && toggleOpen()"
      >
        <span
          v-if="mode === 'candidates'"
          class="shrink-0 rounded px-1.5 py-0.5 text-[10px] tracking-[0.12em]"
          :class="theme.badgeClass"
        >
          匹配
        </span>
        <span class="min-w-0 flex-1 truncate tracking-wide">
          {{ triggerLabel }}
        </span>
        <Icon
          icon="lucide:chevrons-up-down"
          width="14"
          height="14"
          class="shrink-0"
          :class="theme.chevronClass"
        />
      </button>

      <div
        v-if="open && hasOptions"
        class="absolute top-[calc(100%+0.35rem)] z-50 max-h-52 overflow-y-auto rounded-xl border p-1 backdrop-blur-md scrollbar-none"
        :class="[
          theme.menuClass,
          compact ? 'right-0 w-[min(100vw-2.5rem,20rem)]' : 'left-0 w-full min-w-[18rem]',
        ]"
      >
        <LyricsSelectOptionRow
          v-for="(option, index) in options"
          :key="option.id"
          :option="option"
          :index="index"
          :active="option.id === selectedOption?.id"
          :title-class="theme.titleClass.value"
          :meta-class="theme.metaClass.value"
          :index-class="theme.indexClass.value"
          :item-hover-class="theme.itemHoverClass.value"
          :item-active-class="theme.itemActiveClass.value"
          @select="handleSelect"
        />
      </div>
    </div>

    <a-button
      v-if="mode === 'search'"
      type="primary"
      :loading="searchState?.searching.value"
      :disabled="disabled || searchDisabled"
      @click="handleSearch()"
    >
      检索
    </a-button>
  </div>
</template>
