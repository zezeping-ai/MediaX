<script setup lang="ts">
const props = defineProps<{
  timeline: Array<{ stage: string; message: string; at_ms: number }>;
}>();

function formatStageLabel(stage: string) {
  switch (stage) {
    case "open":
      return "打开";
    case "decoder_ready":
      return "解码器";
    case "video_stream":
      return "视频流";
    case "audio":
      return "音频流";
    case "running":
      return "运行";
    case "seek":
      return "跳转";
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
  <div class="rounded-lg border border-slate-400/16 bg-slate-900/20 px-1.5 py-1.5">
    <div class="mb-0.5 text-slate-400/95">解析过程</div>
    <div v-if="!props.timeline.length" class="text-slate-100/70">等待解析信息...</div>
    <div v-else class="space-y-1">
      <div
        v-for="(item, idx) in props.timeline.slice(-60).reverse()"
        :key="`${item.at_ms}-${idx}`"
        class="rounded-md border border-slate-400/12 bg-slate-900/15 px-1.5 py-1"
      >
        <div class="flex items-center justify-between gap-2">
          <span class="text-[10px] text-slate-400/95">{{ formatStageLabel(item.stage) }}</span>
          <span class="text-[10px] text-slate-500/90">{{ formatTime(item.at_ms) }}</span>
        </div>
        <div class="mt-0.5 wrap-break-word text-slate-100/95">{{ item.message }}</div>
      </div>
    </div>
  </div>
</template>

