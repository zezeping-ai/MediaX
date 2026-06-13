<script setup lang="ts">
import type { PlaylistItem } from "@/pages/home/composables/usePlaybackPlaylist/types";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";
import PlaylistItemRow from "./PlaylistItemRow.vue";

defineProps<{
  items: PlaylistItem[];
  currentPlayingId: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  play: [string];
  remove: [string];
  "add-to-queue": [string];
  clear: [];
}>();

const { listFrameOverflow, sectionSubtitle, sectionTitle } = useAppSurfaceTheme();
</script>

<template>
  <section class="space-y-2">
    <div class="flex items-center justify-between gap-3">
      <div>
        <div :class="sectionTitle">播放历史</div>
        <div :class="sectionSubtitle">最近播放的内容，可快速重新打开或加入队列</div>
      </div>
      <a-button v-if="items.length" size="small" danger type="text" @click="emit('clear')">
        清空历史
      </a-button>
    </div>
    <div :class="listFrameOverflow">
      <a-empty v-if="!items.length" description="暂无播放历史" class="py-10" />
      <div v-else class="p-1">
        <PlaylistItemRow
          v-for="item in items"
          :key="item.id"
          :item="item"
          :active="item.id === currentPlayingId"
          :busy="busy"
          @play="emit('play', item.id)"
          @remove="emit('remove', item.id)"
          @add-to-queue="emit('add-to-queue', item.id)"
        />
      </div>
    </div>
  </section>
</template>
