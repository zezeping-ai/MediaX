<script setup lang="ts">
import { ref } from "vue";
import { whenever } from "@vueuse/core";
import { Icon } from "@iconify/vue";
import {
  PLAYBACK_ADVANCE_MODE_OPTIONS,
  type PlaybackAdvanceMode,
  type PlaylistItem,
} from "@/pages/home/composables/usePlaybackPlaylist/types";
import { useAppSurfaceTheme } from "@/pages/home/composables/useAppSurfaceTheme";
import HistoryList from "./HistoryList.vue";
import QueueList from "./QueueList.vue";

const open = defineModel<boolean>("open", { required: true });

defineProps<{
  queue: PlaylistItem[];
  history: PlaylistItem[];
  currentPlayingId: string;
  busy: boolean;
  hasNext: boolean;
  hasPrevious: boolean;
  queueCount: number;
  advanceMode: PlaybackAdvanceMode;
}>();

const emit = defineEmits<{
  playQueue: [string];
  playHistory: [string];
  removeQueue: [string];
  removeHistory: [string];
  addToQueue: [string];
  reorder: [number, number];
  clearQueue: [];
  clearHistory: [];
  playNext: [];
  playPrevious: [];
  importFiles: [];
  importFolder: [];
  changeAdvanceMode: [PlaybackAdvanceMode];
}>();

const activeTab = ref<"queue" | "history">("queue");
const drawerReady = ref(false);
const { countBadge, drawerMaskStyle, insetPanel, sectionCaption } = useAppSurfaceTheme();

function handleAfterOpenChange(nextOpen: boolean) {
  drawerReady.value = nextOpen;
}

whenever(() => !open.value, () => {
  drawerReady.value = false;
});
</script>

<template>
  <a-drawer
    :open="open"
    placement="right"
    width="min(420px, 92vw)"
    :closable="true"
    :push="false"
    :mask-style="drawerMaskStyle"
    root-class-name="playlist-panel-drawer"
    @close="open = false"
    @after-open-change="handleAfterOpenChange"
  >
    <template #title>
      <div class="flex items-center gap-2">
        <Icon icon="lucide:list-music" width="18" height="18" />
        <span>播放列表</span>
        <span v-if="queueCount" :class="countBadge">
          {{ queueCount }}
        </span>
      </div>
    </template>

    <div class="flex flex-col gap-4">
      <div :class="insetPanel">
        <div class="mb-2" :class="sectionCaption">
          播放模式
        </div>
        <div class="grid grid-cols-4 gap-1.5">
          <a-button
            v-for="option in PLAYBACK_ADVANCE_MODE_OPTIONS"
            :key="option.value"
            size="small"
            :disabled="busy"
            :type="advanceMode === option.value ? 'primary' : 'default'"
            class="flex! h-auto min-h-9 flex-col items-center justify-center gap-0.5 px-1 py-1.5"
            :title="option.title"
            @click="emit('changeAdvanceMode', option.value)"
          >
            <Icon :icon="option.icon" width="14" height="14" aria-hidden="true" />
            <span class="text-[10px] leading-none">{{ option.label }}</span>
          </a-button>
        </div>
      </div>

      <div class="flex flex-wrap items-center gap-2">
        <a-button size="small" :disabled="busy" @click="emit('importFiles')">
          添加文件
        </a-button>
        <a-button size="small" :disabled="busy" @click="emit('importFolder')">
          导入文件夹
        </a-button>
        <a-button size="small" :disabled="busy || !hasPrevious" @click="emit('playPrevious')">
          上一项
        </a-button>
        <a-button size="small" type="primary" :disabled="busy || !hasNext" @click="emit('playNext')">
          下一项
        </a-button>
      </div>

      <a-tabs v-model:active-key="activeTab" size="small">
        <a-tab-pane key="queue" tab="接下来">
          <QueueList
            :items="queue"
            :current-playing-id="currentPlayingId"
            :busy="busy"
            :sortable-enabled="drawerReady && activeTab === 'queue'"
            @play="emit('playQueue', $event)"
            @remove="emit('removeQueue', $event)"
            @reorder="(oldIndex, newIndex) => emit('reorder', oldIndex, newIndex)"
            @clear="emit('clearQueue')"
          />
        </a-tab-pane>
        <a-tab-pane key="history" tab="历史">
          <HistoryList
            :items="history"
            :current-playing-id="currentPlayingId"
            :busy="busy"
            @play="emit('playHistory', $event)"
            @remove="emit('removeHistory', $event)"
            @add-to-queue="emit('addToQueue', $event)"
            @clear="emit('clearHistory')"
          />
        </a-tab-pane>
      </a-tabs>
    </div>
  </a-drawer>
</template>
