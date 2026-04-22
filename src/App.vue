<script setup lang="ts">
import { computed } from "vue";
import { theme as antdTheme } from "ant-design-vue";
import { usePreferences } from "./modules/preferences";

const { resolvedTheme } = usePreferences();

const configTheme = computed(() => ({
  algorithm:
    resolvedTheme.value === "dark"
      ? antdTheme.darkAlgorithm
      : antdTheme.defaultAlgorithm,
}));
</script>

<template>
  <a-config-provider :theme="configTheme">
    <router-view />
  </a-config-provider>
</template>
