<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Icon } from "@iconify/vue";
import AppearanceSection from "../sections/appearance/index.vue";
import PlayerSection from "../sections/player/index.vue";
import { usePreferences } from "@/modules/preferences";

type SectionKey = "appearance" | "player";

type Section = {
  key: SectionKey;
  label: string;
  icon: string;
  component: any;
};

const SECTIONS: Section[] = [
  {
    key: "appearance",
    label: "外观",
    icon: "mdi:palette-outline",
    component: AppearanceSection,
  },
  {
    key: "player",
    label: "播放器",
    icon: "mdi:play-circle-outline",
    component: PlayerSection,
  },
];

const router = useRouter();
const route = useRoute();
const { resolvedTheme } = usePreferences();
const siderTheme = computed(() => (resolvedTheme.value === "dark" ? "dark" : "light"));

const activeKey = computed<SectionKey>({
  get: () =>
    (typeof route.query.section === "string" && (route.query.section as SectionKey)) ||
    "appearance",
  set: (v) => {
    router.replace({
      query: {
        ...route.query,
        section: v,
      },
    });
  },
});

const selectedKeys = computed<string[]>({
  get: () => [activeKey.value],
  set: (keys) => {
    const k = (keys?.[0] || "appearance") as SectionKey;
    activeKey.value = k;
  },
});

const activeSection = computed(() => SECTIONS.find((s) => s.key === activeKey.value) ?? SECTIONS[0]);
</script>

<template>
  <a-layout class="h-full">
    <a-layout-sider
      width="200"
      :theme="siderTheme"
      class="border-r border-black/6 dark:border-white/10"
    >
      <div class="px-3 pb-2 pt-3">
        <a-typography-title :level="5" class="m-0!">设置</a-typography-title>
        <a-typography-text type="secondary" class="text-[11px]">
          偏好会自动保存
        </a-typography-text>
      </div>

      <a-menu v-model:selectedKeys="selectedKeys" mode="inline">
        <a-menu-item v-for="s in SECTIONS" :key="s.key">
          <a-space>
            <Icon :icon="s.icon" aria-hidden="true" />
            <span>{{ s.label }}</span>
          </a-space>
        </a-menu-item>
      </a-menu>
    </a-layout-sider>

    <a-layout class="h-full">
      <a-layout-content class="h-full p-4 overflow-auto">
        <component :is="activeSection.component" />
      </a-layout-content>
    </a-layout>
  </a-layout>
</template>

