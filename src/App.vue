<script setup lang="ts">
import { computed, watch } from "vue";
import { theme as antdTheme } from "ant-design-vue";
import { useRoute } from "vue-router";
import { playbackSetDebugLogEnabled } from "@/modules/media-player";
import { usePreferences } from "@/modules/preferences";

const { resolvedTheme, playerDebugLogEnabled } = usePreferences();
const route = useRoute();

const configTheme = computed(() => ({
  algorithm:
    resolvedTheme.value === "dark"
      ? antdTheme.darkAlgorithm
      : antdTheme.defaultAlgorithm,
}));

const componentSize = computed(() => (route.name === "preferences" ? "small" : "middle"));

watch(
  playerDebugLogEnabled,
  (enabled) => {
    void playbackSetDebugLogEnabled(enabled).catch(() => {
      // 日志偏好不应阻塞应用启动；失败时保持静默。
    });
  },
  { immediate: true },
);

watch(() => route.name, () => {
  document.documentElement.dataset.page = String(route.name ?? "");
}, { immediate: true });
</script>

<template>
  <a-config-provider :theme="configTheme" :component-size="componentSize">
    <router-view />
  </a-config-provider>
</template>
