<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, ref, toRef } from "vue";
import type { PlaybackChannelRouting } from "@/modules/media-types";
import {
  CIRCLE_BTN_BASE,
  CIRCLE_BTN_GHOST,
  CIRCLE_BTN_PRIMARY,
  PILL_BASE,
  SPEED_OPTIONS,
  TINY_PILL_BTN,
} from "../playbackControls.constants";
import { type PlaybackQualityOption } from "../playbackControlsUtils";
import {
  channelRoutingLabel,
  channelStateLabel,
  formatPercent,
} from "./channelTrimDisplay";
import { useChannelTrimPanel } from "./useChannelTrimPanel";

const props = defineProps<{
  disabled: boolean;
  isPlaying: boolean;
  playbackRate: number;
  selectedQuality: string;
  qualityLabel: string;
  qualityOptions: PlaybackQualityOption[];
  muted: boolean;
  volume: number;
  volumeIcon: string;
  leftChannelVolume: number;
  rightChannelVolume: number;
  leftChannelMuted: boolean;
  rightChannelMuted: boolean;
  channelRouting: PlaybackChannelRouting;
  speedDropdownOpen: boolean;
  qualityDropdownOpen: boolean;
}>();

const emit = defineEmits<{
  play: [];
  pause: [];
  stop: [];
  "toggle-speed-open": [boolean];
  "toggle-quality-open": [boolean];
  "change-speed": [string | number];
  "change-quality": [string | number];
  "toggle-mute": [];
  "overlay-interaction-change": [boolean];
  "change-volume": [number | [number, number]];
  "commit-volume": [number | [number, number]];
  "set-left-channel-volume": [number];
  "set-right-channel-volume": [number];
  "set-left-channel-muted": [boolean];
  "set-right-channel-muted": [boolean];
  "set-channel-routing": [PlaybackChannelRouting];
}>();

const rootRef = ref<HTMLElement | null>(null);
const {
  channelPanelOpen,
  leftVolumePreview,
  rightVolumePreview,
  toggleChannelPanel,
} = useChannelTrimPanel({
  rootRef,
  leftChannelVolume: toRef(props, "leftChannelVolume"),
  rightChannelVolume: toRef(props, "rightChannelVolume"),
  speedDropdownOpen: toRef(props, "speedDropdownOpen"),
  qualityDropdownOpen: toRef(props, "qualityDropdownOpen"),
  emitOverlayInteractionChange: (open) => emit("overlay-interaction-change", open),
});

const leftChannelSummary = computed(() => {
  if (props.leftChannelMuted) {
    return "M";
  }
  return formatPercent(leftVolumePreview.value);
});

const rightChannelSummary = computed(() => {
  if (props.rightChannelMuted) {
    return "M";
  }
  return formatPercent(rightVolumePreview.value);
});

const leftEffectiveOutput = computed(() => {
  const master = props.muted ? 0 : props.volume;
  const trim = props.leftChannelMuted ? 0 : leftVolumePreview.value;
  return master * trim;
});

const rightEffectiveOutput = computed(() => {
  const master = props.muted ? 0 : props.volume;
  const trim = props.rightChannelMuted ? 0 : rightVolumePreview.value;
  return master * trim;
});

function updateLeftChannelVolume(value: number | [number, number]) {
  const normalized = Array.isArray(value) ? value[0] : value;
  leftVolumePreview.value = normalized;
  emit("set-left-channel-volume", normalized);
}

function updateRightChannelVolume(value: number | [number, number]) {
  const normalized = Array.isArray(value) ? value[0] : value;
  rightVolumePreview.value = normalized;
  emit("set-right-channel-volume", normalized);
}
</script>

<template>
  <div class="flex justify-center">
    <div ref="rootRef" class="relative">
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
            <Icon icon="mdi:chevron-up" width="14" height="14" class="shrink-0 opacity-75" aria-hidden="true" />
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
            <Icon icon="mdi:chevron-up" width="14" height="14" class="shrink-0 opacity-75" aria-hidden="true" />
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

        <div class="ml-1 mr-2 w-[156px] max-[720px]:hidden">
          <div class="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.16em] text-white/42">
            <span>Master</span>
            <span>{{ formatPercent(muted ? 0 : volume) }}</span>
          </div>
          <a-slider
            class="w-full [&_.ant-slider]:m-0! [&_.ant-slider-handle::after]:bg-white [&_.ant-slider-handle::after]:shadow-[0_0_0_2px_rgba(255,255,255,0.20)] [&_.ant-slider-handle:hover]:opacity-100 [&_.ant-slider-handle]:opacity-90 [&_.ant-slider-rail]:h-[3px] [&_.ant-slider-rail]:bg-white/12 [&_.ant-slider-track]:h-[3px] [&_.ant-slider-track]:bg-white/70"
            :value="muted ? 0 : volume"
            :min="0"
            :max="1"
            :step="0.01"
            :tip-formatter="formatPercent"
            :disabled="disabled"
            title="调整音量"
            @update:value="$emit('change-volume', $event)"
            @change="$emit('change-volume', $event)"
            @afterChange="$emit('commit-volume', $event)"
          />
          <div class="mt-1.5 flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-white/36">
            <span>L {{ leftChannelSummary }}</span>
            <span>{{ channelRoutingLabel(channelRouting) }}</span>
            <span>R {{ rightChannelSummary }}</span>
          </div>
        </div>

        <a-button
          size="small"
          :disabled="disabled"
          :class="[TINY_PILL_BTN, channelPanelOpen ? 'bg-white/14 text-white' : '']"
          :title="channelPanelOpen ? '收起声道控制' : '展开声道控制'"
          @click="toggleChannelPanel"
        >
          <Icon icon="lucide:audio-lines" width="14" height="14" class="shrink-0" aria-hidden="true" />
          <span>L/R</span>
        </a-button>
      </div>

      <div
        v-if="channelPanelOpen"
        class="absolute bottom-[calc(100%+12px)] left-1/2 z-20 w-[min(340px,calc(100vw-32px))] -translate-x-1/2 rounded-2xl border border-white/12 bg-[linear-gradient(180deg,rgba(8,8,10,0.94)_0%,rgba(14,14,18,0.90)_100%)] p-3 shadow-[0_18px_48px_rgba(0,0,0,0.42)] backdrop-blur-2xl"
      >
        <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-white/50">
          <span>Channel Trim</span>
          <span class="text-white/28">{{ channelRoutingLabel(channelRouting) }}</span>
        </div>

        <div class="mb-3 rounded-xl border border-white/8 bg-white/[0.03] p-2.5">
          <div class="mb-2 flex items-center justify-between text-[11px] text-white/78">
            <div class="flex items-center gap-2">
              <span>Routing</span>
              <span class="text-[10px] uppercase tracking-[0.16em] text-white/38">Source Map</span>
            </div>
            <span class="text-[10px] uppercase tracking-[0.14em] text-white/42">
              {{ channelRoutingLabel(channelRouting) }}
            </span>
          </div>
          <a-segmented
            block
            size="small"
            :disabled="disabled"
            :value="channelRouting"
            :options="[
              { label: 'Stereo', value: 'stereo' },
              { label: 'L->LR', value: 'left_to_both' },
              { label: 'R->LR', value: 'right_to_both' },
            ]"
            @change="$emit('set-channel-routing', $event as PlaybackChannelRouting)"
          />
        </div>

        <div class="space-y-3">
          <div class="rounded-xl border border-white/8 bg-white/[0.03] px-3 py-2.5">
            <div class="mb-2 flex items-center justify-between text-[11px] text-white/78">
              <div class="flex items-center gap-2">
                <span>L</span>
                <span class="text-[10px] uppercase tracking-[0.16em] text-white/38">Trim</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-[10px] uppercase tracking-[0.14em] text-white/42">
                  out {{ formatPercent(leftEffectiveOutput) }}
                </span>
                <a-button
                  size="small"
                  :disabled="disabled"
                  :class="[TINY_PILL_BTN, 'h-7 px-2.5 text-[11px]']"
                  @click="$emit('set-left-channel-muted', !leftChannelMuted)"
                >
                  {{ leftChannelMuted ? "Unmute" : "Mute" }}
                </a-button>
              </div>
            </div>
            <div class="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-white/34">
              <span>trim {{ formatPercent(leftChannelMuted ? 0 : leftVolumePreview) }}</span>
              <span>{{ channelStateLabel(leftChannelMuted) }}</span>
            </div>
            <div class="px-1">
              <a-slider
                class="w-full"
                :value="leftChannelMuted ? 0 : leftVolumePreview"
                :min="0"
                :max="1"
                :step="0.01"
                :tip-formatter="formatPercent"
                :disabled="disabled"
                title="调整左声道音量"
                @update:value="updateLeftChannelVolume"
                @change="updateLeftChannelVolume"
              />
            </div>
          </div>

          <div class="rounded-xl border border-white/8 bg-white/[0.03] px-3 py-2.5">
            <div class="mb-2 flex items-center justify-between text-[11px] text-white/78">
              <div class="flex items-center gap-2">
                <span>R</span>
                <span class="text-[10px] uppercase tracking-[0.16em] text-white/38">Trim</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-[10px] uppercase tracking-[0.14em] text-white/42">
                  out {{ formatPercent(rightEffectiveOutput) }}
                </span>
                <a-button
                  size="small"
                  :disabled="disabled"
                  :class="[TINY_PILL_BTN, 'h-7 px-2.5 text-[11px]']"
                  @click="$emit('set-right-channel-muted', !rightChannelMuted)"
                >
                  {{ rightChannelMuted ? "Unmute" : "Mute" }}
                </a-button>
              </div>
            </div>
            <div class="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-white/34">
              <span>trim {{ formatPercent(rightChannelMuted ? 0 : rightVolumePreview) }}</span>
              <span>{{ channelStateLabel(rightChannelMuted) }}</span>
            </div>
            <div class="px-1">
              <a-slider
                class="w-full"
                :value="rightChannelMuted ? 0 : rightVolumePreview"
                :min="0"
                :max="1"
                :step="0.01"
                :tip-formatter="formatPercent"
                :disabled="disabled"
                title="调整右声道音量"
                @update:value="updateRightChannelVolume"
                @change="updateRightChannelVolume"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
