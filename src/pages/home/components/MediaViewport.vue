<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { type PlaybackState } from "@/modules/media-types";
import { usePreferences } from "@/modules/preferences";

const props = defineProps<{
  source: string;
  loading: boolean;
  playback: PlaybackState | null;
  debugSnapshot: Record<string, string>;
}>();

const emit = defineEmits<{
  ended: [];
  "quick-open-local": [];
  "quick-open-url": [];
}>();

const { playerParseDebugEnabled } = usePreferences();
const debugDismissedSource = ref("");

const canShowDebugOverlay = computed(
  () =>
    playerParseDebugEnabled.value &&
    Boolean(props.source) &&
    debugDismissedSource.value !== props.source,
);

function closeCurrentDebugOverlay() {
  if (!props.source) {
    return;
  }
  debugDismissedSource.value = props.source;
}

const hwDecodeLabel = computed(() => {
  const p = props.playback;
  if (!p) {
    return "";
  }
  const active = p.hw_decode_active ? "on" : "off";
  const backend = p.hw_decode_backend || "<none>";
  const mode = p.hw_decode_mode || "auto";
  const err = p.hw_decode_error ? ` | err=${p.hw_decode_error}` : "";
  return `hw_decode mode=${mode} active=${active} backend=${backend}${err}`;
});

const debugRows = computed(() => {
  const preferredOrder = [
    "open",
    "decoder_ready",
    "video_stream",
    "audio",
    "running",
    "video_pipeline",
    "telemetry",
    "video_fps",
    "audio_stats",
    "video_gap",
    "seek",
    "audio_resume",
    "decode_error",
  ];
  const rows: Array<{ key: string; label: string; value: string }> = [];
  for (const key of preferredOrder) {
    const value = props.debugSnapshot[key];
    if (!value) continue;
    rows.push({ key, label: formatDebugLabel(key), value });
  }
  for (const [key, value] of Object.entries(props.debugSnapshot)) {
    if (!value || preferredOrder.includes(key)) continue;
    rows.push({ key, label: formatDebugLabel(key), value });
  }
  if (!rows.length) {
    return [{ key: "empty", label: "status", value: "等待解析信息..." }];
  }
  return rows;
});

function formatDebugLabel(key: string): string {
  const labels: Record<string, string> = {
    open: "打开源",
    decoder_ready: "解码器",
    video_stream: "视频流",
    audio: "音频流",
    running: "运行状态",
    video_pipeline: "视频管线",
    telemetry: "时序指标",
    video_fps: "视频帧率",
    audio_stats: "音频统计",
    video_gap: "帧间间隔",
    seek: "跳转",
    audio_resume: "音频恢复",
    decode_error: "解码错误",
    stop: "停止",
  };
  return labels[key] || key;
}

watch(
  () => props.source,
  (nextSource) => {
    if (!nextSource) {
      emit("ended");
      debugDismissedSource.value = "";
    }
  },
);
</script>

<template>
  <section class="player-canvas">
    <div v-if="source" class="video-underlay" />
    <div v-else class="empty-actions">
      <a-empty description="请从 File 菜单打开本地文件或 URL">
        <template #default>
          <a-space>
            <a-button type="primary" @click="emit('quick-open-local')">打开本地文件</a-button>
            <a-button @click="emit('quick-open-url')">打开 URL</a-button>
          </a-space>
        </template>
      </a-empty>
    </div>
    <a-spin v-if="loading" class="busy-overlay" />

    <div v-if="canShowDebugOverlay" class="debug-overlay">
      <div class="debug-header">
        <div class="debug-title-wrap">
          <div class="debug-title">解析 Debug</div>
          <span class="debug-badge">LIVE</span>
        </div>
        <a-button class="debug-close-btn" size="mini" type="text" @click="closeCurrentDebugOverlay">
          关闭
        </a-button>
      </div>
      <div v-if="hwDecodeLabel" class="debug-meta-row">
        <span class="debug-meta-label">Decoder</span>
        <span class="debug-meta-value">{{ hwDecodeLabel }}</span>
      </div>
      <div class="debug-log-wrap">
        <div class="debug-log-title">实时状态</div>
        <div v-for="row in debugRows" :key="row.key" class="debug-row">
          <span class="debug-row-key">{{ row.label }}</span>
          <span class="debug-row-value">{{ row.value }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.player-canvas {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  background: transparent;
  overflow: hidden;
}

.busy-overlay {
  position: absolute;
}

.empty-actions {
  padding: 20px;
}

.video-underlay {
  width: 100%;
  height: 100%;
}

.debug-overlay {
  position: absolute;
  left: 16px;
  top: 16px;
  width: min(560px, calc(100vw - 32px));
  max-height: min(55vh, 420px);
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow: hidden;
  z-index: 5;
  padding: 10px 10px 10px 12px;
  border-radius: 12px;
  background: linear-gradient(180deg, rgba(11, 16, 23, 0.86) 0%, rgba(9, 13, 20, 0.78) 100%);
  border: 1px solid rgba(255, 255, 255, 0.16);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.28);
  backdrop-filter: blur(14px);
  color: rgba(241, 245, 249, 0.95);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New",
    monospace;
  font-size: 11px;
  line-height: 16px;
}

.debug-log-wrap {
  flex: 1;
  min-height: 0;
  overflow: auto;
  border-radius: 8px;
  padding: 6px 8px;
  background: transparent;
  border: 1px solid transparent;
  scrollbar-width: thin;
  scrollbar-color: rgba(236, 242, 255, 0.16) transparent;
}

.debug-log-wrap::-webkit-scrollbar {
  width: 7px;
  height: 7px;
}

.debug-log-wrap::-webkit-scrollbar-track {
  background: transparent;
}

.debug-log-wrap::-webkit-scrollbar-thumb {
  border-radius: 999px;
  border: 2px solid transparent;
  background-clip: padding-box;
  background: rgba(236, 242, 255, 0.14);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  transition: background-color 220ms ease, box-shadow 220ms ease;
}

.debug-log-wrap:hover::-webkit-scrollbar-thumb {
  background: rgba(236, 242, 255, 0.34);
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.16);
}

.debug-title {
  font-weight: 700;
  letter-spacing: 0.2px;
}

.debug-title-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
}

.debug-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: 16px;
  padding: 0 6px;
  border-radius: 999px;
  font-size: 10px;
  color: #c7f9cc;
  background: rgba(59, 130, 246, 0.2);
  border: 1px solid rgba(59, 130, 246, 0.45);
}

.debug-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.debug-close-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: rgba(255, 255, 255, 0.86);
  padding: 0 8px;
  height: 22px;
  min-width: 46px;
  line-height: 1;
  border-radius: 6px;
}

.debug-close-btn:hover {
  color: #fff;
  background: rgba(255, 255, 255, 0.12);
}

.debug-meta-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  background: rgba(148, 163, 184, 0.08);
  border: 1px solid rgba(148, 163, 184, 0.2);
}

.debug-meta-label {
  flex-shrink: 0;
  color: rgba(148, 163, 184, 0.95);
}

.debug-meta-value {
  color: rgba(241, 245, 249, 0.95);
  word-break: break-word;
}

.debug-log-title {
  margin-bottom: 4px;
  color: rgba(148, 163, 184, 0.95);
}

.debug-row {
  display: grid;
  grid-template-columns: 88px 1fr;
  gap: 8px;
  align-items: start;
  margin-bottom: 4px;
  opacity: 0.94;
}

.debug-row-key {
  color: rgba(148, 163, 184, 0.95);
  text-transform: lowercase;
}

.debug-row-value {
  color: rgba(241, 245, 249, 0.95);
  word-break: break-word;
}

.debug-muted {
  opacity: 0.65;
}

.debug-hw {
  opacity: 0.85;
  margin-bottom: 4px;
}
</style>
