<script setup lang="ts">
import { computed } from "vue";
import { usePreferences } from "@/modules/preferences";

const props = defineProps<{
  source: string;
  networkReadBytesPerSecond: number | null;
  networkSustainRatio: number | null;
  cacheRecording: boolean;
  cacheOutputPath: string;
  cacheOutputSizeBytes: number | null;
  cacheWriteSpeedBytesPerSecond: number | null;
}>();

const { playerShowDownlinkSpeed, playerShowUplinkSpeed } = usePreferences();

function isUrlSource(value: string) {
  const v = (value || "").trim().toLowerCase();
  return (
    v.startsWith("http://") ||
    v.startsWith("https://") ||
    v.startsWith("rtsp://") ||
    v.startsWith("rtmp://") ||
    v.startsWith("mms://")
  );
}

function formatBytes(value: number | null) {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    return "0 B";
  }
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = value;
  let unitIdx = 0;
  while (size >= 1024 && unitIdx < units.length - 1) {
    size /= 1024;
    unitIdx += 1;
  }
  const precision = size >= 100 || unitIdx === 0 ? 0 : size >= 10 ? 1 : 2;
  return `${size.toFixed(precision)} ${units[unitIdx]}`;
}

const canShowDownlinkSpeed = computed(
  () =>
    playerShowDownlinkSpeed.value
    && Boolean(props.source)
    && isUrlSource(props.source)
    && props.networkReadBytesPerSecond !== null,
);
const canShowUplinkSpeed = computed(
  () =>
    playerShowUplinkSpeed.value
    && Boolean(props.cacheRecording && props.cacheOutputPath),
);
const downlinkSpeedText = computed(() => `${formatBytes(props.networkReadBytesPerSecond)}/s`);
const uplinkSpeedText = computed(() => `${formatBytes(props.cacheWriteSpeedBytesPerSecond)}/s`);
const compactSpeedRows = computed(() => {
  const rows: Array<{
    key: "downlink" | "uplink";
    label: "下行" | "上行";
    value: string;
    toneClass: string;
    title: string;
  }> = [];

  if (canShowDownlinkSpeed.value) {
    rows.push({
      key: "downlink",
      label: "下行",
      value: downlinkSpeedText.value,
      toneClass: resolveDownlinkToneClass(props.networkSustainRatio),
      title: resolveDownlinkHint(props.networkSustainRatio),
    });
  }

  if (canShowUplinkSpeed.value) {
    rows.push({
      key: "uplink",
      label: "上行",
      value: uplinkSpeedText.value,
      toneClass:
        "border-sky-400/35 bg-sky-400/12 text-sky-100 shadow-[inset_0_0_0_1px_rgba(56,189,248,0.14)]",
      title: "缓存/录制写入速度",
    });
  }

  return rows;
});

const canShowCacheRecording = computed(() => Boolean(props.cacheRecording && props.cacheOutputPath));
const cachePathShort = computed(() => {
  const full = props.cacheOutputPath || "";
  if (full.length <= 42) return full;
  return `${full.slice(0, 14)}…${full.slice(-24)}`;
});

function resolveDownlinkToneClass(value: number | null) {
  const ratio = typeof value === "number" && Number.isFinite(value) ? Math.max(0, value) : null;
  if (ratio === null) {
    return "border-white/15 bg-white/8 text-white/90 shadow-[inset_0_0_0_1px_rgba(255,255,255,0.08)]";
  }
  if (ratio >= 1.0) {
    return "border-emerald-400/35 bg-emerald-400/12 text-emerald-50 shadow-[inset_0_0_0_1px_rgba(52,211,153,0.14)]";
  }
  if (ratio >= 0.85) {
    return "border-amber-300/35 bg-amber-400/12 text-amber-50 shadow-[inset_0_0_0_1px_rgba(251,191,36,0.12)]";
  }
  return "border-rose-400/35 bg-rose-400/12 text-rose-50 shadow-[inset_0_0_0_1px_rgba(251,113,133,0.12)]";
}

function resolveDownlinkHint(value: number | null) {
  const ratio = typeof value === "number" && Number.isFinite(value) ? Math.max(0, value) : null;
  if (ratio === null) {
    return "暂未拿到足够的实时吞吐/消费速率样本";
  }
  if (ratio >= 1.0) {
    return `当前下行已覆盖实时播放需求 (${ratio.toFixed(2)}x)`;
  }
  if (ratio >= 0.85) {
    return `当前下行接近实时播放需求 (${ratio.toFixed(2)}x)，可能轻微卡顿`;
  }
  return `当前下行低于实时播放需求 (${ratio.toFixed(2)}x)，更容易缓冲或卡顿`;
}
</script>

<template>
  <div
    v-if="canShowDownlinkSpeed || canShowUplinkSpeed || canShowCacheRecording"
    class="pointer-events-none absolute right-4 top-4 z-20 flex max-w-[min(360px,calc(100vw-32px))] flex-col gap-2"
  >
    <div
      v-if="compactSpeedRows.length"
      class="rounded-xl bg-slate-950/30 px-2.5 py-2 text-[12px] leading-4 text-white/90 shadow-[0_10px_30px_rgba(0,0,0,0.25)] backdrop-blur-md"
    >
      <div class="flex flex-wrap items-center gap-2">
        <div
          v-for="row in compactSpeedRows"
          :key="row.key"
          :title="row.title"
          :class="[
            'inline-flex items-center gap-1.5 rounded-full border px-2 py-1 text-[11px] leading-none',
            row.toneClass,
          ]"
        >
          <span class="font-medium tracking-[0.2px]">{{ row.label }}</span>
          <span class="tabular-nums opacity-95">{{ row.value }}</span>
        </div>
      </div>
    </div>

    <div
      v-if="canShowCacheRecording"
      class="rounded-xl border border-white/12 bg-slate-950/30 px-3 py-2 text-[12px] leading-4 text-white/90 shadow-[0_10px_30px_rgba(0,0,0,0.25)] backdrop-blur-md"
    >
      <div class="mb-1 flex items-center justify-between gap-3">
        <span class="text-white/65">录制写入</span>
        <span class="tabular-nums text-white/95">
          {{ formatBytes(cacheOutputSizeBytes) }}
          <span class="text-white/50">·</span>
          {{ formatBytes(cacheWriteSpeedBytesPerSecond) }}/s
        </span>
      </div>
      <div class="text-[11px] text-white/55" :title="cacheOutputPath">{{ cachePathShort }}</div>
    </div>
  </div>
</template>
