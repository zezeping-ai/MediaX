<script setup lang="ts">
import type { UrlPlaylistItem } from "./types";

defineProps<{
  playlist: UrlPlaylistItem[];
  busy: boolean;
}>();

const emit = defineEmits<{
  clear: [];
  remove: [string];
  select: [string];
  play: [string];
}>();

function formatOpenedAt(timestamp: number) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return "未知时间";
  }
  return new Date(timestamp).toLocaleString();
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center justify-between">
      <span class="text-xs opacity-70">播放列表（最近优先）</span>
      <a-button
        v-if="playlist.length"
        size="small"
        danger
        type="text"
        @click="emit('clear')"
      >
        一键清空
      </a-button>
    </div>
    <a-empty v-if="!playlist.length" description="暂无历史 URL" />
    <a-list v-else size="small" :data-source="playlist">
      <template #renderItem="{ item }">
        <a-list-item class="overflow-hidden">
          <div class="min-w-0 w-full space-y-1 overflow-hidden">
            <button
              class="block min-w-0 w-full cursor-pointer bg-transparent p-0 text-left"
              type="button"
              :title="item.url"
              @click="emit('select', item.url)"
            >
              <span
                class="block min-w-0 w-full overflow-hidden text-ellipsis whitespace-nowrap text-xs text-[rgba(255,255,255,0.85)]"
              >
                {{ item.url }}
              </span>
            </button>
            <div class="flex items-center justify-between gap-2">
              <span class="text-xs opacity-70">{{ formatOpenedAt(item.lastOpenedAt) }}</span>
              <a-space :size="4">
                <a-button size="small" type="link" :disabled="busy" @click="emit('play', item.url)">
                  播放
                </a-button>
                <a-button size="small" danger type="text" @click="emit('remove', item.url)">
                  删除
                </a-button>
              </a-space>
            </div>
          </div>
        </a-list-item>
      </template>
    </a-list>
  </div>
</template>
