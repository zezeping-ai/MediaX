<script setup lang="ts">
import { usePreferences, type ThemePreference } from "@/modules/preferences";

const { theme } = usePreferences();

const options: Array<{ label: string; value: ThemePreference; description: string }> = [
  { label: "跟随系统", value: "system", description: "根据系统浅色/深色自动切换" },
  { label: "暗黑", value: "dark", description: "始终使用深色主题" },
  { label: "明亮", value: "light", description: "始终使用浅色主题" },
];
</script>

<template>
  <a-space direction="vertical" size="small" class="w-full">
    <a-typography-title :level="5" class="m-0!">外观</a-typography-title>
    <a-typography-text type="secondary" class="text-[12px]">
      这里的设置会立即生效，并同步保存到本地。
    </a-typography-text>

    <a-card title="主题" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-radio-group v-model:value="theme" class="w-full">
        <a-space direction="vertical" size="small" class="w-full">
          <a-radio v-for="o in options" :key="o.value" :value="o.value">
            <a-space direction="vertical" size="small">
              <span>{{ o.label }}</span>
              <a-typography-text type="secondary" class="text-xs">
                {{ o.description }}
              </a-typography-text>
            </a-space>
          </a-radio>
        </a-space>
      </a-radio-group>
    </a-card>
  </a-space>
</template>

