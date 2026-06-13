<script setup lang="ts">
import { computed, nextTick, onMounted, ref, toRef } from "vue";
import { watchPausable } from "@vueuse/core";
import type { PlaylistItem } from "@/pages/home/composables/usePlaybackPlaylist/types";
import { reorderListItems } from "@/pages/home/composables/usePlaybackPlaylist/playlistHelpers";
import { useSortableList } from "@/pages/home/composables/useSortableList";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";
import PlaylistItemRow from "./PlaylistItemRow.vue";

const props = defineProps<{
  items: PlaylistItem[];
  currentPlayingId: string;
  busy: boolean;
  sortableEnabled?: boolean;
}>();

const emit = defineEmits<{
  play: [string];
  remove: [string];
  reorder: [number, number];
  clear: [];
}>();

const scrollRef = ref<HTMLElement | null>(null);
const listRef = ref<HTMLElement | null>(null);
const sortableEnabled = toRef(props, "sortableEnabled");
const displayItems = ref<PlaylistItem[]>([]);

const dragSyncPausers = {
  pause: () => {},
  resume: () => {},
};

const itemsSync = watchPausable(
  () => props.items,
  (items) => {
    displayItems.value = items.slice();
  },
  { immediate: true, deep: true },
);

function applyLocalReorder(oldIndex: number, newIndex: number) {
  const next = reorderListItems(displayItems.value, oldIndex, newIndex);
  if (!next) {
    return;
  }
  displayItems.value = next;
  emit("reorder", oldIndex, newIndex);
}

const { remount } = useSortableList({
  containerRef: listRef,
  scrollRef,
  enabled: computed(() => sortableEnabled.value !== false),
  handle: ".playlist-drag-handle",
  getItemIds: () => displayItems.value.map((item) => item.id),
  onDragStateChange: (dragging) => {
    if (dragging) {
      itemsSync.pause();
      dragSyncPausers.pause();
      return;
    }
    itemsSync.resume();
    dragSyncPausers.resume();
  },
  onReorder: applyLocalReorder,
});

const displayItemIds = computed(() => displayItems.value.map((item) => item.id).join("\0"));

const remountSync = watchPausable(displayItemIds, async () => {
  await nextTick();
  await remount();
});

dragSyncPausers.pause = remountSync.pause;
dragSyncPausers.resume = remountSync.resume;

const { listFrame, sectionSubtitle, sectionTitle } = useAppSurfaceTheme();

onMounted(() => {
  scrollRef.value = listRef.value?.closest(".ant-drawer-body") ?? null;
});
</script>

<template>
  <section class="space-y-2">
    <div class="flex items-center justify-between gap-3">
      <div>
        <div :class="sectionTitle">接下来播放</div>
        <div :class="sectionSubtitle">拖动手柄可调整播放顺序</div>
      </div>
      <a-button v-if="items.length" size="small" danger type="text" @click="emit('clear')">
        清空队列
      </a-button>
    </div>
    <div :class="listFrame">
      <a-empty v-if="!items.length" description="队列为空，打开媒体后会自动加入" class="py-10" />
      <div v-else class="p-1">
        <div ref="listRef" class="flex flex-col">
          <div
            v-for="item in displayItems"
            :key="item.id"
            data-sortable-item
            :data-id="item.id"
            class="playlist-sortable-item"
          >
            <PlaylistItemRow
              :item="item"
              :active="item.id === currentPlayingId"
              :busy="busy"
              draggable
              @play="emit('play', item.id)"
              @remove="emit('remove', item.id)"
            />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
