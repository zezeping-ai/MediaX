<script setup lang="ts">
import type {
  DebugGroup,
} from "@/pages/home/composables/usePlayerDebugOverlay";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import DebugGroupSections from "../DebugGroupSections.vue";
import PipelineHealthPanel from "../PipelineHealthPanel";

defineProps<{
  source: string;
  playback: PlaybackState | null;
  telemetry: MediaTelemetryPayload | null;
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
  sections: DebugGroup[];
}>();
</script>

<template>
  <PipelineHealthPanel :source="source" :playback="playback" :telemetry="telemetry" :history="history" />
  <DebugGroupSections
    title="输入供给 / 视频处理 / 音频 / 渲染 管线"
    :groups="sections"
    empty-text="等待管线运行数据..."
  />
</template>
