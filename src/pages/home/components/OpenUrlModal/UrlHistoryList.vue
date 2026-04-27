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
  <section class="space-y-2">
    <div class="flex items-center justify-between gap-3">
      <div>
        <div class="text-sm font-medium text-white/88">播放列表</div>
        <div class="text-xs text-white/42">最近打开优先，点击地址可快速回填</div>
      </div>
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
    <div class="overflow-hidden rounded-lg border border-white/8">
      <a-empty v-if="!playlist.length" description="暂无历史 URL" class="py-8" />
      <a-list
        v-else
        size="small"
        :data-source="playlist"
        class="max-h-[20rem] overflow-y-auto"
      >
      <template #renderItem="{ item }">
        <a-list-item class="overflow-hidden px-3 py-2.5 transition-colors hover:bg-white/[0.03]">
          <div class="min-w-0 w-full space-y-1.5 overflow-hidden">
            <button
              class="block min-w-0 w-full cursor-pointer bg-transparent p-0 text-left"
              type="button"
              :title="item.url"
              @click="emit('select', item.url)"
            >
              <span
                class="block min-w-0 w-full overflow-hidden text-ellipsis whitespace-nowrap text-sm text-[rgba(255,255,255,0.88)]"
              >
                {{ item.url }}
              </span>
            </button>
            <div class="flex items-center justify-between gap-3">
              <span class="truncate text-[11px] text-white/42">{{ formatOpenedAt(item.lastOpenedAt) }}</span>
              <a-space :size="2">
                <a-button size="small" type="link" :disabled="busy" @click="emit('play', item.url)">
                  打开
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
  </section>
</template>
