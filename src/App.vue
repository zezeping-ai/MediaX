<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { theme as antdTheme } from "ant-design-vue";
import { useRoute, useRouter } from "vue-router";
import { MEDIA_MENU_EVENT } from "@/modules/media-types";
import { setRendererBackdropTheme } from "@/modules/media-player/windowCommands";
import { usePreferences } from "@/modules/preferences";

const { resolvedTheme } = usePreferences();
const route = useRoute();
const router = useRouter();
let unlistenMenuEvent: UnlistenFn | null = null;

const configTheme = computed(() => ({
  algorithm:
    resolvedTheme.value === "dark"
      ? antdTheme.darkAlgorithm
      : antdTheme.defaultAlgorithm,
}));

const componentSize = computed(() => "small");

watch(() => route.name, () => {
  document.documentElement.dataset.page = String(route.name ?? "");
}, { immediate: true });

watch(
  resolvedTheme,
  (theme) => {
    void setRendererBackdropTheme(theme);
  },
  { immediate: true },
);

void listen<string>(MEDIA_MENU_EVENT, async (event) => {
  const action = event.payload;
  if (action === "open_local" || action === "open_url") {
    await router.push({
      path: "/",
      query: {
        ...route.query,
        menuAction: action,
      },
    });
    return;
  }
}).then((unlisten) => {
  unlistenMenuEvent = unlisten;
});

onBeforeUnmount(() => {
  unlistenMenuEvent?.();
  unlistenMenuEvent = null;
});
</script>

<template>
  <a-config-provider :theme="configTheme" :component-size="componentSize">
    <router-view />
  </a-config-provider>
</template>
