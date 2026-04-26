<script setup lang="ts">
import type { ProcessStage } from "@/pages/home/composables/usePlayerDebugOverlay";
import ParseProcessPanel from "../ParseProcessPanel.vue";
import ProcessStagePanel from "../ProcessStagePanel.vue";

defineProps<{
  stages: ProcessStage[];
  timeline: Array<{ stage: string; message: string; at_ms: number }>;
}>();
</script>

<template>
  <ProcessStagePanel :stages="stages" />
  <ParseProcessPanel :timeline="timeline" />
</template>
