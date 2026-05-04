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
  confirm: [string];
  cancel: [];
  clear: [];
  remove: [string];
  select: [string];
  play: [string];
}>();

function setOpen(value: boolean) {
  emit("update:open", value);
}

function updateInputValue(value: string | undefined | null) {
  emit("update:inputValue", typeof value === "string" ? value : "");
}

function canConfirmInput(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function emitConfirmIfValid(value: unknown) {
  if (!canConfirmInput(value)) {
    return;
  }
  emit("confirm", value.trim());
}
</script>

<template>
  <a-modal
    :open="props.open"
    title="打开 URL"
    :footer="null"
    :confirm-loading="props.busy"
    @cancel="emit('cancel')"
    @update:open="setOpen"
  >
    <div class="space-y-4">
      <div class="space-y-2">
        <div class="flex items-center justify-between gap-3">
          <div>
            <div class="text-sm font-medium text-white/88">媒体直链</div>
            <div class="text-xs text-white/42">
              支持 http(s)、rtsp、rtmp、mms
            </div>
          </div>
        </div>
        <a-input-group compact class="flex">
          <a-input
            :value="props.inputValue"
            class="flex-1"
            placeholder="输入媒体 URL 或流地址"
            allow-clear
            @update:value="updateInputValue"
            @press-enter="emitConfirmIfValid(props.inputValue)"
          />
          <a-button
            type="primary"
            :loading="props.busy"
            :disabled="!canConfirmInput(props.inputValue)"
            @click="emitConfirmIfValid(props.inputValue)"
          >
            打开
          </a-button>
        </a-input-group>
      </div>
      <UrlHistoryList
        :playlist="props.playlist"
        :busy="props.busy"
        @clear="emit('clear')"
        @remove="(url) => emit('remove', url)"
        @select="(url) => emit('select', url)"
        @play="(url) => emit('play', url)"
      />
    </div>
  </a-modal>
</template>
