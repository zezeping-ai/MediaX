<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  source: string;
  timeline: Array<{ stage: string; message: string; at_ms: number }>;
}>();

const recentTimeline = computed(() => props.timeline.slice(-8).reverse());

function formatStageLabel(stage: string) {
  switch (stage) {
    case "open":
      return "打开源";
    case "stream_start":
      return "启动流";
    case "video_demux":
      return "视频解复用";
    case "video_gop":
      return "GOP 分析";
    case "decoder_ready":
      return "解码器就绪";
    case "audio":
      return "音频流";
    case "video_stream":
      return "视频流";
    case "running":
      return "进入播放";
    default:
      return stage;
  }
}

function formatTime(ms: number) {
  const d = new Date(ms);
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  const mss = String(d.getMilliseconds()).padStart(3, "0");
  return `${hh}:${mm}:${ss}.${mss}`;
}
</script>

<template>
  <div
    class="absolute left-4 top-4 z-20 flex w-[min(520px,calc(100vw-24px))] max-w-[calc(100vw-24px)] flex-col gap-2 overflow-hidden rounded-xl border border-cyan-300/18 bg-[linear-gradient(180deg,rgba(7,16,24,0.92)_0%,rgba(8,13,20,0.84)_100%)] px-3 py-3 font-mono text-[11px] leading-4 text-slate-100 shadow-[0_18px_60px_rgba(0,0,0,0.35)] backdrop-blur-[16px]"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <div class="font-semibold tracking-[0.2px] text-cyan-100">解析过程</div>
          <span class="inline-flex items-center rounded-full border border-cyan-300/25 bg-cyan-400/10 px-1.5 py-0.5 text-[10px] text-cyan-100">
            首帧前
          </span>
        </div>
        <div class="mt-1 truncate text-[10px] text-slate-400/90" :title="source">{{ source }}</div>
      </div>
      <a-spin size="small" />
    </div>

    <div class="h-1 overflow-hidden rounded-full bg-white/8">
      <div class="loading-progress-bar h-full w-1/3 rounded-full bg-[linear-gradient(90deg,rgba(34,211,238,0.18)_0%,rgba(34,211,238,0.95)_55%,rgba(125,211,252,0.18)_100%)]" />
    </div>

    <div class="space-y-1">
      <div v-if="!recentTimeline.length" class="rounded-md border border-white/8 bg-white/[0.03] px-2 py-2 text-slate-300/75">
        等待拉起输入源与解析流程...
      </div>
      <div
        v-for="(item, idx) in recentTimeline"
        :key="`${item.at_ms}-${idx}`"
        class="rounded-md border border-white/8 bg-white/[0.03] px-2 py-1.5"
      >
        <div class="flex items-center justify-between gap-2 text-[10px] text-slate-400/95">
          <span>{{ formatStageLabel(item.stage) }}</span>
          <span>{{ formatTime(item.at_ms) }}</span>
        </div>
        <div class="mt-0.5 wrap-break-word text-slate-100/92">{{ item.message }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.loading-progress-bar {
  animation: loading-progress-slide 1.35s ease-in-out infinite;
}

@keyframes loading-progress-slide {
  0% {
    transform: translateX(-120%);
  }
  100% {
    transform: translateX(320%);
  }
}
</style>
