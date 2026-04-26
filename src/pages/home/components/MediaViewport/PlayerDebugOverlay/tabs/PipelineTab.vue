<script setup lang="ts">
import type {
  DebugGroup,
} from "@/pages/home/composables/usePlayerDebugOverlay";
import type { MediaTelemetryPayload } from "@/modules/media-types";
import DebugGroupSections from "../DebugGroupSections.vue";
import PipelineHealthPanel from "../PipelineHealthPanel.vue";

defineProps<{
  telemetry: MediaTelemetryPayload | null;
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
  sections: DebugGroup[];
}>();
</script>

<template>
  <PipelineHealthPanel :telemetry="telemetry" :history="history" />
  <DebugGroupSections
    title="输入 / 解码 / 音频 / 渲染 管线"
    :groups="sections"
    empty-text="等待管线运行数据..."
  />
</template>
