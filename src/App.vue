<script setup lang="ts">
import { computed, watchEffect } from "vue";
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

watchEffect(() => {
  document.documentElement.dataset.page = String(route.name ?? "");
});
</script>

<template>
  <a-config-provider :theme="configTheme" :component-size="componentSize">
    <router-view />
  </a-config-provider>
</template>
