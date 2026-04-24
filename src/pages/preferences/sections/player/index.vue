<script setup lang="ts">
import { usePreferences } from "@/modules/preferences";
import { setMediaHwDecodeMode } from "@/modules/media-player";

const { playerHwDecodeEnabled, playerParseDebugEnabled } = usePreferences();

async function applyHwDecode(enabled: boolean) {
  // “开”使用 auto：尽可能启用硬解，失败则自动回退。
  const mode = enabled ? "auto" : "off";
  try {
    await setMediaHwDecodeMode(mode);
  } catch {
    // 不把错误强行冒泡到偏好页；播放页会显示具体错误事件。
  }
}
</script>

<template>
  <a-space direction="vertical" size="middle" style="width: 100%">
    <a-typography-title :level="4" style="margin: 0">播放器</a-typography-title>
    <a-typography-text type="secondary">
      这些设置会自动保存，并在播放时生效。
    </a-typography-text>

    <a-card title="解码" :bordered="false">
      <a-space direction="vertical" style="width: 100%">
        <div class="setting-row">
          <div class="setting-text">
            <div class="setting-title">硬件解码</div>
            <div class="setting-desc">开启后会优先尝试使用系统硬解加速（不支持时自动回退）。</div>
          </div>
          <a-switch
            v-model:checked="playerHwDecodeEnabled"
            @change="(checked: boolean) => void applyHwDecode(checked)"
          />
        </div>
      </a-space>
    </a-card>

    <a-card title="调试" :bordered="false">
      <a-space direction="vertical" style="width: 100%">
        <div class="setting-row">
          <div class="setting-text">
            <div class="setting-title">解析 Debug</div>
            <div class="setting-desc">在播放器上展示加载/解析过程以及调试信息（默认开启）。</div>
          </div>
          <a-switch v-model:checked="playerParseDebugEnabled" />
        </div>
      </a-space>
    </a-card>
  </a-space>
</template>

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.setting-title {
  font-weight: 600;
}

.setting-desc {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.55);
}

:global([data-theme="dark"] .setting-desc) {
  color: rgba(255, 255, 255, 0.55);
}
</style>

