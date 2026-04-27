<script setup lang="ts">
import { onMounted, ref } from "vue";
import { message } from "ant-design-vue";
import { Icon } from "@iconify/vue";
import { usePreferences } from "@/modules/preferences";
import {
  playbackClearDebugLog,
  playbackConfigureDecoderMode,
  playbackGetDebugLogPath,
} from "@/modules/media-player";
import {
  applyAlwaysOnTopPreference,
  applyVideoScaleModePreference,
} from "@/modules/player-settings-actions";
import type { HardwareDecodeMode } from "@/modules/media-types";

const {
  playerHwDecodeMode,
  playerParseDebugEnabled,
  playerDebugLogEnabled,
  playerAlwaysOnTop,
  playerVideoScaleMode,
  playerShowDownlinkSpeed,
  playerShowUplinkSpeed,
} = usePreferences();

const debugLogPath = ref("");
const loadingDebugLogPath = ref(false);
const clearingDebugLog = ref(false);

async function applyHwDecode(mode: HardwareDecodeMode) {
  try {
    await playbackConfigureDecoderMode(mode);
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

async function loadDebugLogPath() {
  loadingDebugLogPath.value = true;
  try {
    debugLogPath.value = await playbackGetDebugLogPath();
  } catch {
    debugLogPath.value = "";
  } finally {
    loadingDebugLogPath.value = false;
  }
}

async function copyDebugLogPath() {
  if (!debugLogPath.value) return;
  try {
    await navigator.clipboard.writeText(debugLogPath.value);
    message.success("日志位置已复制");
  } catch {
    message.error("复制日志位置失败");
  }
}

async function clearDebugLog() {
  clearingDebugLog.value = true;
  try {
    debugLogPath.value = await playbackClearDebugLog();
    message.success("日志已清空");
  } catch {
    message.error("清空日志失败");
  } finally {
    clearingDebugLog.value = false;
  }
}

onMounted(() => {
  void loadDebugLogPath();
});
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
              Auto 会尽量优先使用硬解，失败时自动回退软解，并在重新起流时再次尝试。
            </div>
          </div>
          <a-segmented
            v-model:value="playerHwDecodeMode"
            :options="[
              { label: 'Auto', value: 'auto' },
              { label: 'On', value: 'on' },
              { label: 'Off', value: 'off' },
            ]"
            @change="(value: string | number) => void applyHwDecode(value as HardwareDecodeMode)"
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

    <a-card title="日志管理" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full" size="middle">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">播放日志</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              启动时自动清空当前日志，运行中超过 1MB 自动轮转，默认开启。
            </div>
          </div>
          <a-switch v-model:checked="playerDebugLogEnabled" />
        </div>

        <div class="flex min-w-0 flex-col gap-2">
          <div class="text-xs font-semibold uppercase tracking-[0.12em] text-black/45 dark:text-white/45">
            位置
          </div>
          <a-input
            :value="debugLogPath"
            readonly
            :loading="loadingDebugLogPath"
            placeholder="正在读取日志位置..."
          />
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <a-button size="small" @click="void loadDebugLogPath()">
            <template #icon>
              <Icon icon="mdi:refresh" />
            </template>
            刷新位置
          </a-button>
          <a-button size="small" :disabled="!debugLogPath" @click="void copyDebugLogPath()">
            <template #icon>
              <Icon icon="mdi:content-copy" />
            </template>
            复制位置
          </a-button>
          <a-button size="small" danger :loading="clearingDebugLog" @click="void clearDebugLog()">
            <template #icon>
              <Icon icon="mdi:delete-outline" />
            </template>
            一键清空
          </a-button>
        </div>
      </a-space>
    </a-card>

    <a-card title="网络速率" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">显示下行</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              在播放器右上角显示网络读取速度，标签简写为“下行”。
            </div>
          </div>
          <a-switch v-model:checked="playerShowDownlinkSpeed" />
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">显示上行</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              在播放器右上角显示缓存/录制写入速度，标签简写为“上行”。
            </div>
          </div>
          <a-switch v-model:checked="playerShowUplinkSpeed" />
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
