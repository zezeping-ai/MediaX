<script setup lang="ts">
import type { CurrentFrameSection } from "@/pages/home/composables/usePlayerDebugOverlay";
import CurrentFramePanel from "../CurrentFramePanel.vue";

defineProps<{
  sections: CurrentFrameSection[];
}>();
</script>

<template>
  <CurrentFramePanel :sections="sections" />
</template>
