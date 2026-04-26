<script setup lang="ts">
import type {
  DebugGroup,
  HardwareDecisionEvent,
} from "@/pages/home/composables/usePlayerDebugOverlay";
import DecodeStatusBanner from "../DecodeStatusBanner.vue";
import DebugGroupSections from "../DebugGroupSections.vue";
import HardwareDecisionHistoryPanel from "../HardwareDecisionHistoryPanel.vue";
import MediaInfoPanel from "../MediaInfoPanel.vue";

defineProps<{
  mediaInfoGroups: Array<{
    id: string;
    title: string;
    rows: Array<{ key: string; label: string; value: string }>;
  }>;
  decodeBanner: {
    backend: string;
    mode: string;
    modeLabel: string;
    error: string | null;
  } | null;
  hardwareDecisionTimeline: HardwareDecisionEvent[];
  overviewSections: DebugGroup[];
}>();
</script>

<template>
  <MediaInfoPanel :groups="mediaInfoGroups" />
  <DecodeStatusBanner
    v-if="decodeBanner"
    :backend="decodeBanner.backend"
    :mode="decodeBanner.mode"
    :mode-label="decodeBanner.modeLabel"
    :error="decodeBanner.error"
  />
  <HardwareDecisionHistoryPanel :events="hardwareDecisionTimeline" />
  <DebugGroupSections
    title="会话与解码概览"
    :groups="overviewSections"
    empty-text="等待会话与解码概览数据..."
  />
</template>
