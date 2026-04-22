<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Icon } from "@iconify/vue";
import AppearanceSection from "../sections/appearance/index.vue";

type SectionKey = "appearance";

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
];

const router = useRouter();
const route = useRoute();

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
  <a-layout style="min-height: 100vh">
    <a-layout-sider
      width="220"
      theme="light"
      style="border-right: 1px solid rgba(5, 5, 5, 0.06)"
    >
      <div style="padding: 16px 16px 8px">
        <a-typography-title :level="4" style="margin: 0">设置</a-typography-title>
        <a-typography-text type="secondary" style="font-size: 12px">
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

    <a-layout>
      <a-layout-content style="padding: 24px">
        <component :is="activeSection.component" />
      </a-layout-content>
    </a-layout>
  </a-layout>
</template>

