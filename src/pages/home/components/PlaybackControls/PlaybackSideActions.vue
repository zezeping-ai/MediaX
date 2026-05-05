<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST } from "./playbackControls.constants";

defineProps<{
  cacheRecording: boolean;
  locked: boolean;
  showAudioExport: boolean;
  cacheIcon: string;
  lockIcon: string;
}>();

defineEmits<{
  "toggle-cache": [];
  "toggle-lock": [];
  "export-audio": [];
}>();
</script>

<template>
  <div>
    <div class="flex items-center gap-1.5">
      <a-button
        v-if="showAudioExport"
        type="text"
        size="small"
        shape="circle"
        class="max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
        :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST]"
        title="导出视频音频到文件"
        @click="$emit('export-audio')"
      >
        <Icon icon="lucide:file-audio-2" width="15" height="15" class="block shrink-0" aria-hidden="true" />
      </a-button>

    <a-button
      type="text"
      size="small"
      shape="circle"
      class="max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
      :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST, cacheRecording ? 'bg-[#1677ff33] text-[#91caff]' : '']"
      :title="cacheRecording ? '停止缓存录制' : '开始缓存录制到文件'"
      @click="$emit('toggle-cache')"
    >
      <Icon :icon="cacheIcon" width="15" height="15" class="block shrink-0" aria-hidden="true" />
    </a-button>

    <a-button
      type="text"
      size="small"
      shape="circle"
      class="max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
      :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST, locked ? 'bg-white/15 text-white' : '']"
      :title="locked ? '取消锁定控制器自动隐藏' : '锁定控制器常驻显示'"
      @click="$emit('toggle-lock')"
    >
      <Icon :icon="lockIcon" width="15" height="15" class="block shrink-0" aria-hidden="true" />
    </a-button>
    </div>
  </div>
</template>
