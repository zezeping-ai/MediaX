<script setup lang="ts">
type UrlPlaylistItem = {
  url: string;
  lastOpenedAt: number;
};

const props = defineProps<{
  open: boolean;
  inputValue: string;
  playlist: UrlPlaylistItem[];
  busy: boolean;
}>();

const emit = defineEmits<{
  "update:open": [boolean];
  "update:inputValue": [string];
  confirm: [];
  cancel: [];
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

function setOpen(value: boolean) {
  emit("update:open", value);
}
</script>

<template>
  <a-modal
    :open="props.open"
    title="打开 URL"
    ok-text="开始播放"
    cancel-text="取消"
    :confirm-loading="props.busy"
    @ok="emit('confirm')"
    @cancel="emit('cancel')"
    @update:open="setOpen"
  >
    <a-space direction="vertical" class="w-full" :size="12">
      <a-input
        :value="props.inputValue"
        placeholder="请输入视频 URL（http/https）"
        allow-clear
        @update:value="(value: string) => emit('update:inputValue', value)"
        @press-enter="emit('confirm')"
      />
      <div class="flex items-center justify-between">
        <span class="text-xs opacity-70">播放列表（最近优先）</span>
        <a-button
          v-if="props.playlist.length"
          size="small"
          danger
          type="text"
          @click="emit('clear')"
        >
          一键清空
        </a-button>
      </div>
      <a-empty v-if="!props.playlist.length" description="暂无历史 URL" />
      <a-list v-else size="small" :data-source="props.playlist">
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
                  <a-button
                    size="small"
                    type="link"
                    :disabled="props.busy"
                    @click="emit('play', item.url)"
                  >
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
    </a-space>
  </a-modal>
</template>

