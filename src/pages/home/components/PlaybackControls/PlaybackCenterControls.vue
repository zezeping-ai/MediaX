<script setup lang="ts">
import { Icon } from "@iconify/vue";
import {
  CIRCLE_BTN_BASE,
  CIRCLE_BTN_GHOST,
  CIRCLE_BTN_PRIMARY,
  PILL_BASE,
  SPEED_OPTIONS,
  TINY_PILL_BTN,
} from "./playbackControls.constants";
import { type PlaybackQualityOption } from "./playbackControlsUtils";

defineProps<{
  disabled: boolean;
  isPlaying: boolean;
  playbackRate: number;
  selectedQuality: string;
  qualityLabel: string;
  qualityOptions: PlaybackQualityOption[];
  muted: boolean;
  volume: number;
  volumeIcon: string;
  speedDropdownOpen: boolean;
  qualityDropdownOpen: boolean;
}>();

defineEmits<{
  play: [];
  pause: [];
  stop: [];
  "toggle-speed-open": [boolean];
  "toggle-quality-open": [boolean];
  "change-speed": [string | number];
  "change-quality": [string | number];
  "toggle-mute": [];
  "change-volume": [number | [number, number]];
  "commit-volume": [number | [number, number]];
}>();
</script>

<template>
  <div class="flex justify-center">
    <div :class="[PILL_BASE, 'max-w-full gap-2.5 px-3']">
      <a-button
        size="small"
        shape="circle"
        :disabled="disabled"
        :title="isPlaying ? '暂停并保留当前画面' : '停止播放并清空当前画面'"
        :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST]"
        @click="$emit('stop')"
      >
        <Icon icon="ph:stop-fill" width="16" height="16" class="block shrink-0" aria-hidden="true" />
      </a-button>

      <a-button
        size="small"
        shape="circle"
        :disabled="disabled"
        :title="isPlaying ? '暂停播放' : '开始播放'"
        :class="[CIRCLE_BTN_BASE, 'h-11 min-h-11 w-11 min-w-11', CIRCLE_BTN_PRIMARY]"
        @click="isPlaying ? $emit('pause') : $emit('play')"
      >
        <Icon
          :icon="isPlaying ? 'ph:pause-fill' : 'ph:play-fill'"
          width="20"
          height="20"
          class="block shrink-0"
          aria-hidden="true"
        />
      </a-button>

      <span class="h-5 w-px bg-white/10" aria-hidden="true" />

      <a-dropdown
        :open="speedDropdownOpen"
        :trigger="['click']"
        placement="top"
        @update:open="$emit('toggle-speed-open', $event)"
      >
        <a-button size="small" :class="TINY_PILL_BTN" :disabled="disabled" title="调整播放倍速">
          <span class="tabular-nums">{{ playbackRate }}x</span>
          <Icon
            icon="mdi:chevron-up"
            width="14"
            height="14"
            class="shrink-0 opacity-75"
            aria-hidden="true"
          />
        </a-button>
        <template #overlay>
          <a-menu :selected-keys="[String(playbackRate)]" @click="$emit('change-speed', $event.key)">
            <a-menu-item v-for="value in SPEED_OPTIONS" :key="String(value)">
              {{ value }}x
            </a-menu-item>
          </a-menu>
        </template>
      </a-dropdown>

      <span class="h-5 w-px bg-white/10" aria-hidden="true" />

      <a-dropdown
        :open="qualityDropdownOpen"
        :trigger="['click']"
        placement="top"
        @update:open="$emit('toggle-quality-open', $event)"
      >
        <a-button
          size="small"
          :class="TINY_PILL_BTN"
          :disabled="disabled || qualityOptions.length <= 1"
          title="切换清晰度"
        >
          <span class="tabular-nums">{{ qualityLabel }}</span>
          <Icon
            icon="mdi:chevron-up"
            width="14"
            height="14"
            class="shrink-0 opacity-75"
            aria-hidden="true"
          />
        </a-button>
        <template #overlay>
          <a-menu :selected-keys="[selectedQuality]" @click="$emit('change-quality', $event.key)">
            <a-menu-item v-for="option in qualityOptions" :key="option.key">
              {{ option.label }}
            </a-menu-item>
          </a-menu>
        </template>
      </a-dropdown>

      <span class="h-5 w-px bg-white/10" aria-hidden="true" />

      <a-button
        size="small"
        shape="circle"
        :disabled="disabled"
        :title="muted || volume <= 0 ? '取消静音' : '静音'"
        :class="[CIRCLE_BTN_BASE, CIRCLE_BTN_GHOST]"
        @click="$emit('toggle-mute')"
      >
        <Icon :icon="volumeIcon" width="18" height="18" class="block shrink-0" aria-hidden="true" />
      </a-button>

      <div class="ml-1 mr-2 w-[118px] max-[720px]:hidden">
        <a-slider
          class="w-full [&_.ant-slider]:m-0! [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.20)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-90 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:h-[3px] [&_.ant-slider-track]:bg-white/70"
          :value="muted ? 0 : volume"
          :min="0"
          :max="1"
          :step="0.01"
          :tooltip-open="false"
          :disabled="disabled"
          title="调整音量"
          @update:value="$emit('change-volume', $event)"
          @change="$emit('change-volume', $event)"
          @afterChange="$emit('commit-volume', $event)"
        />
      </div>
    </div>
  </div>
</template>
