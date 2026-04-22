<script setup lang="ts">
import { usePreferences, type ThemePreference } from "../../../../modules/preferences";

const { theme } = usePreferences();

const options: Array<{ label: string; value: ThemePreference; description: string }> = [
  { label: "跟随系统", value: "system", description: "根据系统浅色/深色自动切换" },
  { label: "暗黑", value: "dark", description: "始终使用深色主题" },
  { label: "明亮", value: "light", description: "始终使用浅色主题" },
];
</script>

<template>
  <a-space direction="vertical" size="middle" style="width: 100%">
    <a-typography-title :level="4" style="margin: 0">外观</a-typography-title>
    <a-typography-text type="secondary">
      这里的设置会立即生效，并同步保存到本地。
    </a-typography-text>

    <a-card title="主题" :bordered="false">
      <a-radio-group v-model:value="theme" style="width: 100%">
        <a-space direction="vertical" style="width: 100%">
          <a-radio v-for="o in options" :key="o.value" :value="o.value">
            <a-space direction="vertical" size="small">
              <span>{{ o.label }}</span>
              <a-typography-text type="secondary" style="font-size: 12px">
                {{ o.description }}
              </a-typography-text>
            </a-space>
          </a-radio>
        </a-space>
      </a-radio-group>
    </a-card>
  </a-space>
</template>

