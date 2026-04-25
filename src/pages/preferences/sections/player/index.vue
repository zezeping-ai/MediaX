<script setup lang="ts">
import { usePreferences } from "@/modules/preferences";
import { setMediaHwDecodeMode } from "@/modules/media-player";
import {
  applyAlwaysOnTopPreference,
  applyVideoScaleModePreference,
} from "@/modules/player-settings-actions";

const { playerHwDecodeEnabled, playerParseDebugEnabled, playerAlwaysOnTop, playerVideoScaleMode } =
  usePreferences();

async function applyHwDecode(enabled: boolean) {
  // “开”使用 auto：尽可能启用硬解，失败则自动回退。
  const mode = enabled ? "auto" : "off";
  try {
    await setMediaHwDecodeMode(mode);
  } catch {
    // 不把错误强行冒泡到偏好页；播放页会显示具体错误事件。
  }
}

async function applyAlwaysOnTop(enabled: boolean) {
  await applyAlwaysOnTopPreference(enabled);
}

async function applyVideoScaleMode(mode: "contain" | "cover") {
  await applyVideoScaleModePreference(mode);
}
</script>

<template>
  <a-space direction="vertical" size="small" class="w-full">
    <a-typography-title :level="5" class="m-0!">播放器</a-typography-title>
    <a-typography-text type="secondary" class="text-[12px]">
      这些设置会自动保存，并在播放时生效。
    </a-typography-text>

    <a-card title="解码" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">硬件解码</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              开启后会优先尝试使用系统硬解加速（不支持时自动回退）。
            </div>
          </div>
          <a-switch
            v-model:checked="playerHwDecodeEnabled"
            @change="(checked: boolean) => void applyHwDecode(checked)"
          />
        </div>
      </a-space>
    </a-card>

    <a-card title="调试" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">解析 Debug</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              在播放器上展示加载/解析过程以及调试信息（默认开启）。
            </div>
          </div>
          <a-switch v-model:checked="playerParseDebugEnabled" />
        </div>
      </a-space>
    </a-card>

    <a-card title="窗口" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">窗口置顶</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              开启后播放器主窗口将保持在其它窗口上方显示。
            </div>
          </div>
          <a-switch
            v-model:checked="playerAlwaysOnTop"
            @change="(checked: boolean) => void applyAlwaysOnTop(checked)"
          />
        </div>
      </a-space>
    </a-card>

    <a-card title="画面" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">视频显示模式</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              自适应会完整显示视频（留黑边）；铺满会填满窗口（可能裁切画面）。
            </div>
          </div>
          <a-segmented
            v-model:value="playerVideoScaleMode"
            :options="[
              { label: '自适应', value: 'contain' },
              { label: '铺满', value: 'cover' },
            ]"
            @change="(value: string | number) => void applyVideoScaleMode(value as 'contain' | 'cover')"
          />
        </div>
      </a-space>
    </a-card>
  </a-space>
</template>

<style scoped>
/* migrated to Tailwind utility classes */
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

