<script setup lang="ts">
import { computed, watch } from "vue";
import { theme as antdTheme } from "ant-design-vue";
import { useRoute } from "vue-router";
import { usePreferences } from "@/modules/preferences";

const { resolvedTheme } = usePreferences();
const route = useRoute();

const configTheme = computed(() => ({
  algorithm:
    resolvedTheme.value === "dark"
      ? antdTheme.darkAlgorithm
      : antdTheme.defaultAlgorithm,
}));

const componentSize = computed(() => (route.name === "preferences" ? "small" : "middle"));

watch(() => route.name, () => {
  document.documentElement.dataset.page = String(route.name ?? "");
}, { immediate: true });
</script>

<template>
  <a-config-provider :theme="configTheme" :component-size="componentSize">
    <router-view />
  </a-config-provider>
</template>
