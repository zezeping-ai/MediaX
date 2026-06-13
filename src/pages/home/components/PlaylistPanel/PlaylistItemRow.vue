<script setup lang="ts">
import { Icon } from "@iconify/vue";
import type { PlaylistItem } from "@/pages/home/composables/usePlaybackPlaylist/types";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";

defineProps<{
  item: PlaylistItem;
  active: boolean;
  busy: boolean;
  draggable?: boolean;
}>();

const emit = defineEmits<{
  play: [];
  remove: [];
  "add-to-queue": [];
}>();

const { dragHandle, rowHover, rowMeta, rowMuted, rowTitle } = useAppSurfaceTheme();

function formatTime(timestamp: number | null) {
  if (!timestamp || !Number.isFinite(timestamp)) {
    return "";
  }
  return new Date(timestamp).toLocaleString();
}
</script>

<template>
  <div
    class="group flex items-start gap-2 rounded-lg border border-transparent px-2 py-2 transition-colors"
    :class="[rowHover, active ? 'border-[#1677ff55] bg-[#1677ff1f]' : '']"
  >
    <div
      v-if="draggable"
      role="button"
      tabindex="-1"
      class="playlist-drag-handle mt-0.5 flex h-7 w-5 shrink-0 cursor-grab items-center justify-center rounded active:cursor-grabbing"
      :class="dragHandle"
      title="拖动排序"
    >
      <Icon icon="lucide:grip-vertical" width="14" height="14" />
    </div>
    <div class="min-w-0 flex-1">
      <div class="truncate" :class="rowTitle" :title="item.title">
        {{ item.title }}
      </div>
      <div class="mt-0.5 truncate" :class="rowMeta" :title="item.source">
        {{ item.source }}
      </div>
      <div v-if="item.lastPlayedAt" class="mt-1" :class="rowMuted">
        {{ formatTime(item.lastPlayedAt) }}
      </div>
    </div>
    <div class="flex shrink-0 items-center gap-0.5 opacity-80 transition-opacity group-hover:opacity-100">
      <a-button size="small" type="link" :disabled="busy" @click="emit('play')">播放</a-button>
      <a-button
        v-if="!draggable"
        size="small"
        type="link"
        :disabled="busy"
        @click="emit('add-to-queue')"
      >
        加入
      </a-button>
      <a-button size="small" danger type="text" @click="emit('remove')">删除</a-button>
    </div>
  </div>
</template>
