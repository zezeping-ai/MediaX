<script setup lang="ts">
import type { DebugGroup } from "@/pages/home/composables/usePlayerDebugOverlay";
import DebugGroupSections from "../DebugGroupSections.vue";

defineProps<{
  title: string;
  groups: DebugGroup[];
  emptyText: string;
}>();
</script>

<template>
  <DebugGroupSections :title="title" :groups="groups" :empty-text="emptyText" />
</template>
