<script setup lang="ts">
import UrlHistoryList from "./UrlHistoryList.vue";
import type { UrlPlaylistItem } from "./types";

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
      <UrlHistoryList
        :playlist="props.playlist"
        :busy="props.busy"
        @clear="emit('clear')"
        @remove="(url) => emit('remove', url)"
        @select="(url) => emit('select', url)"
        @play="(url) => emit('play', url)"
      />
    </a-space>
  </a-modal>
</template>
