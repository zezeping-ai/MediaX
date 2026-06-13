<script setup lang="ts">
import {
  applyAlwaysOnTopPreference,
  applyLyricsFetchSettingsPreference,
  applyResumeLastPositionPreference,
  applyVideoPictureTunePreference,
  applyVideoScaleModePreference,
} from "@/modules/player-settings-actions";
import { usePreferences } from "@/modules/preferences";
import { normalizeVideoPictureTune, type VideoPictureTune } from "@/modules/video-picture-tune";
import VideoPictureTunePanel from "./VideoPictureTunePanel/index.vue";

const {
  playerHwDecodeMode,
  playerAlwaysOnTop,
  playerVideoScaleMode,
  playerVideoPictureTune,
  playerShowDownlinkSpeed,
  playerShowUplinkSpeed,
  playerResumeLastPosition,
  playerAutoFetchOnlineLyrics,
  playerShowLyrics,
  playerLyricsProviders,
  playerLrcApiBaseUrl,
} = usePreferences();

async function applyAlwaysOnTop(enabled: boolean) {
  await applyAlwaysOnTopPreference(enabled);
}

async function applyVideoScaleMode(mode: "contain" | "cover") {
  await applyVideoScaleModePreference(mode);
}

async function applyVideoPictureTune() {
  await applyVideoPictureTunePreference(normalizeVideoPictureTune(playerVideoPictureTune.value));
}

function updateVideoPictureTune(next: VideoPictureTune) {
  playerVideoPictureTune.value = normalizeVideoPictureTune(next);
}

async function syncLyricsSettings() {
  await applyLyricsFetchSettingsPreference({
    autoFetchOnlineLyrics: playerAutoFetchOnlineLyrics.value,
    providers: playerLyricsProviders.value,
    lrcApiBaseUrl: playerLrcApiBaseUrl.value,
  });
}
</script>

<template>
  <a-space direction="vertical" size="small" class="w-full">
    <a-typography-title :level="5" class="m-0!">播放器</a-typography-title>
    <a-typography-text type="secondary" class="text-[12px]">
      这些设置会自动保存，并在播放时生效。
    </a-typography-text>

    <a-card title="播放" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">记住播放进度</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              退出播放时保存进度；再次打开同一文件时从头播放，并在进度条上方提示是否跳转。
            </div>
          </div>
          <a-switch
            v-model:checked="playerResumeLastPosition"
            @change="(checked: boolean) => void applyResumeLastPositionPreference(checked)"
          />
        </div>
      </a-space>
    </a-card>

    <a-card title="歌词" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <a-space direction="vertical" class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">显示歌词</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              播放音频时展示歌词面板。也可在歌词面板右上角单独隐藏当前歌曲的歌词。
            </div>
          </div>
          <a-switch v-model:checked="playerShowLyrics" />
        </div>

        <div class="flex items-center justify-between gap-4" :class="!playerShowLyrics ? 'opacity-50' : ''">
          <div class="flex min-w-0 flex-col gap-1">
            <div class="font-semibold">自动获取在线歌词</div>
            <div class="text-xs text-black/55 dark:text-white/55">
              本地无歌词时，会并行查询各在线源并展示全部匹配结果供切换；即使已有本地歌词或缓存，也会继续刷新在线匹配。
            </div>
          </div>
          <a-switch
            v-model:checked="playerAutoFetchOnlineLyrics"
            :disabled="!playerShowLyrics"
            @change="() => void syncLyricsSettings()"
          />
        </div>

        <div class="flex items-center justify-between gap-4" :class="!playerAutoFetchOnlineLyrics || !playerShowLyrics ? 'opacity-50' : ''">
          <div class="font-semibold">网易云音乐</div>
          <a-switch
            v-model:checked="playerLyricsProviders.netease"
            :disabled="!playerAutoFetchOnlineLyrics || !playerShowLyrics"
            @change="() => void syncLyricsSettings()"
          />
        </div>

        <div class="flex items-center justify-between gap-4" :class="!playerAutoFetchOnlineLyrics || !playerShowLyrics ? 'opacity-50' : ''">
          <div class="font-semibold">酷狗音乐</div>
          <a-switch
            v-model:checked="playerLyricsProviders.kugou"
            :disabled="!playerAutoFetchOnlineLyrics || !playerShowLyrics"
            @change="() => void syncLyricsSettings()"
          />
        </div>

        <div class="flex items-center justify-between gap-4" :class="!playerAutoFetchOnlineLyrics || !playerShowLyrics ? 'opacity-50' : ''">
          <div class="font-semibold">LRCLIB</div>
          <a-switch
            v-model:checked="playerLyricsProviders.lrclib"
            :disabled="!playerAutoFetchOnlineLyrics || !playerShowLyrics"
            @change="() => void syncLyricsSettings()"
          />
        </div>

        <div class="flex items-center justify-between gap-4" :class="!playerAutoFetchOnlineLyrics || !playerShowLyrics ? 'opacity-50' : ''">
          <div class="font-semibold">LrcApi</div>
          <a-switch
            v-model:checked="playerLyricsProviders.lrcapi"
            :disabled="!playerAutoFetchOnlineLyrics || !playerShowLyrics"
            @change="() => void syncLyricsSettings()"
          />
        </div>

        <div :class="!playerAutoFetchOnlineLyrics || !playerShowLyrics ? 'opacity-50' : ''">
          <div class="mb-1 font-semibold">LrcApi 自定义地址</div>
          <a-input
            v-model:value="playerLrcApiBaseUrl"
            :disabled="!playerAutoFetchOnlineLyrics || !playerShowLyrics"
            placeholder="https://api.lrc.cx"
            @blur="() => void syncLyricsSettings()"
          />
          <div class="mt-1 text-xs text-black/55 dark:text-white/55">
            留空使用官方公开实例。LrcApi 为 GPL-3.0 项目，MediaX 仅通过 HTTP 调用，不嵌入其源码。
          </div>
        </div>
      </a-space>
    </a-card>

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
          />
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

    <a-card title="画面调节" :bordered="false" size="small" :body-style="{ padding: '12px' }">
      <VideoPictureTunePanel
        :tune="playerVideoPictureTune"
        @update:tune="updateVideoPictureTune"
        @change="() => void applyVideoPictureTune()"
      />
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
