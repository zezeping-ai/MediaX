<script setup lang="ts">
import type { MediaItem } from "../../../modules/media";

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
  <aside class="library-panel">
    <div class="panel-header">
      <a-typography-title :level="4">媒体库</a-typography-title>
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
      class="search-input"
      @update:value="(value: string) => emit('update:keyword', value)"
    />
    <a-list :data-source="items" size="small" class="media-list" :loading="loading">
      <template #renderItem="{ item }">
        <a-list-item class="media-item" @click="emit('select-item', item)">
          <a-list-item-meta :title="item.name" :description="item.path" />
        </a-list-item>
      </template>
    </a-list>
  </aside>
</template>

<style scoped>
.library-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border-right: 1px solid rgba(127, 127, 127, 0.2);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-header :deep(h4.ant-typography) {
  margin-bottom: 0;
}

.search-input {
  margin-bottom: 4px;
}

.media-list {
  min-height: 0;
  overflow: auto;
  flex: 1;
}

.media-item {
  cursor: pointer;
}
</style>
