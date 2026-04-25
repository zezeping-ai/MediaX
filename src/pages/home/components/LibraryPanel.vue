<script setup lang="ts">
import type { MediaItem } from "@/modules/media-types";

defineProps<{
  roots: string[];
  items: MediaItem[];
  keyword: string;
  loading: boolean;
}>();

const emit = defineEmits<{
  "update:keyword": [string];
  "pick-roots": [];
  "refresh": [];
  "select-item": [MediaItem];
}>();
</script>

<template>
  <aside class="flex flex-col gap-3 border-r border-slate-500/20 p-4">
    <div class="flex items-center justify-between">
      <a-typography-title :level="4" class="mb-0!">媒体库</a-typography-title>
      <a-space>
        <a-button size="small" @click="emit('pick-roots')">选择目录</a-button>
        <a-button size="small" @click="emit('refresh')">刷新</a-button>
      </a-space>
    </div>
    <a-typography-text type="secondary" v-if="roots.length">
      {{ roots.length }} 个目录已接入
    </a-typography-text>
    <a-input
      :value="keyword"
      placeholder="搜索媒体文件"
      allow-clear
      class="mb-1"
      @update:value="(value: string) => emit('update:keyword', value)"
    />
    <a-list :data-source="items" size="small" class="min-h-0 flex-1 overflow-auto" :loading="loading">
      <template #renderItem="{ item }">
        <a-list-item class="cursor-pointer" @click="emit('select-item', item)">
          <a-list-item-meta :title="item.name" :description="item.path" />
        </a-list-item>
      </template>
    </a-list>
  </aside>
</template>
