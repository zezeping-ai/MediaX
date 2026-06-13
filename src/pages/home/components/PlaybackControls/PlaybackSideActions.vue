<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { usePlayerChromeTheme } from "@/pages/home/composables/usePlayerChromeTheme";
import type { SideActionEmitContract, SideActionViewProps } from "./bindings.contract";

defineProps<SideActionViewProps>();
const { circleBtnBase, circleBtnGhost, isDark } = usePlayerChromeTheme();

defineEmits<SideActionEmitContract>();
</script>

<template>
  <div>
    <div class="flex items-center gap-1.5">
      <a-button
        type="text"
        size="small"
        shape="circle"
        class="relative max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
        :class="[circleBtnBase, circleBtnGhost, playlistOpen ? 'bg-[#1677ff33] text-[#91caff]' : '']"
        title="播放列表"
        @click="$emit('toggle-playlist')"
      >
        <Icon icon="lucide:list-music" width="15" height="15" class="block shrink-0" aria-hidden="true" />
        <span
          v-if="queueCount > 0"
          class="absolute -right-0.5 -top-0.5 min-w-[14px] rounded-full bg-[#1677ff] px-1 text-[9px] leading-[14px] text-white"
        >
          {{ queueCount > 99 ? "99+" : queueCount }}
        </span>
      </a-button>

      <a-button
        v-if="showAudioExport"
        type="text"
        size="small"
        shape="circle"
        class="max-[720px]:h-9 max-[720px]:min-h-9 max-[720px]:w-9 max-[720px]:min-w-9"
        :class="[circleBtnBase, circleBtnGhost]"
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
        :class="[circleBtnBase, circleBtnGhost, cacheRecording ? 'bg-[#1677ff33] text-[#91caff]' : '']"
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
        :class="[circleBtnBase, circleBtnGhost, locked ? (isDark ? 'bg-white/15 text-white' : 'bg-black/8 text-slate-900') : '']"
        :title="locked ? '取消锁定控制器自动隐藏' : '锁定控制器常驻显示'"
        @click="$emit('toggle-lock')"
      >
        <Icon :icon="lockIcon" width="15" height="15" class="block shrink-0" aria-hidden="true" />
      </a-button>
    </div>
  </div>
</template>
