<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, ref, toRef } from "vue";
import { formatPlaybackRate, normalizePlaybackRate } from "@/modules/player-constraints";
import type { PlaybackChannelRouting } from "@/modules/media-types";
import { usePlayerChromeTheme } from "@/pages/home/composables/usePlayerChromeTheme";
import {
  SPEED_OPTIONS,
} from "../playbackControls.constants";
import { type PlaybackQualityOption } from "../playbackControlsUtils";
import {
  channelRoutingLabel,
  channelStateLabel,
  formatPercent,
} from "./channelTrimDisplay";
import { useChannelTrimPanel } from "./useChannelTrimPanel";
import type {
  CenterControlEmitContract,
  CenterControlViewProps,
} from "../bindings.contract";

const props = defineProps<CenterControlViewProps & {
  // keep local generic visible for template autocomplete
  qualityOptions: PlaybackQualityOption[];
  channelRouting: PlaybackChannelRouting;
}>();

const emit = defineEmits<CenterControlEmitContract>();

const {
  channelPanel,
  circleBtnBase,
  circleBtnGhost,
  circleBtnPrimary,
  divider,
  isDark,
  masterSlider,
  pillBase,
  tinyPillBtn,
} = usePlayerChromeTheme();

const rootRef = ref<HTMLElement | null>(null);
const normalizedPlaybackRate = computed(() => normalizePlaybackRate(props.playbackRate));
const playbackRateLabel = computed(() => formatPlaybackRate(props.playbackRate));
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
      <div :class="[pillBase, 'max-w-full']">
        <a-button
          size="small"
          shape="circle"
          :disabled="disabled"
          :title="isPlaying ? '暂停并保留当前画面' : '停止播放并清空当前画面'"
          :class="[circleBtnBase, circleBtnGhost]"
          @click="$emit('stop')"
        >
          <Icon icon="ph:stop-fill" width="15" height="15" class="block shrink-0" aria-hidden="true" />
        </a-button>

        <a-button
          size="small"
          shape="circle"
          :disabled="disabled || !hasPrevious"
          title="上一首"
          :class="[circleBtnBase, circleBtnGhost]"
          @click="$emit('play-previous')"
        >
          <Icon icon="ph:skip-back-fill" width="15" height="15" class="block shrink-0" aria-hidden="true" />
        </a-button>

        <a-button
          size="small"
          shape="circle"
          :disabled="disabled"
          :title="isPlaying ? '暂停播放' : '开始播放'"
          :class="[circleBtnBase, circleBtnPrimary]"
          @click="isPlaying ? $emit('pause') : $emit('play')"
        >
          <Icon
            :icon="isPlaying ? 'ph:pause-fill' : 'ph:play-fill'"
            width="16"
            height="16"
            class="block shrink-0"
            aria-hidden="true"
          />
        </a-button>

        <a-button
          size="small"
          shape="circle"
          :disabled="disabled || !hasNext"
          title="下一首"
          :class="[circleBtnBase, circleBtnGhost]"
          @click="$emit('play-next')"
        >
          <Icon icon="ph:skip-forward-fill" width="15" height="15" class="block shrink-0" aria-hidden="true" />
        </a-button>

        <span :class="divider" aria-hidden="true" />

        <a-dropdown
          :open="speedDropdownOpen"
          :trigger="['click']"
          placement="top"
          @update:open="$emit('toggle-speed-open', $event)"
        >
          <a-button size="small" :class="tinyPillBtn" :disabled="disabled" title="调整播放倍速">
            <span class="tabular-nums">{{ playbackRateLabel }}</span>
            <Icon icon="mdi:chevron-up" width="14" height="14" class="shrink-0 opacity-75" aria-hidden="true" />
          </a-button>
          <template #overlay>
            <a-menu :selected-keys="[String(normalizedPlaybackRate)]" @click="$emit('change-speed', $event.key)">
              <a-menu-item v-for="value in SPEED_OPTIONS" :key="String(value)">
                {{ value }}x
              </a-menu-item>
            </a-menu>
          </template>
        </a-dropdown>

        <template v-if="qualityOptions.length > 1">
          <span :class="divider" aria-hidden="true" />

          <a-dropdown
            :open="qualityDropdownOpen"
            :trigger="['click']"
            placement="top"
            @update:open="$emit('toggle-quality-open', $event)"
          >
            <a-button
              size="small"
              :class="tinyPillBtn"
              :disabled="disabled"
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
        </template>

        <span :class="divider" aria-hidden="true" />

        <div class="inline-flex shrink-0 items-center gap-1">
          <a-button
            size="small"
            shape="circle"
            :disabled="disabled"
            :title="muted || volume <= 0 ? '取消静音' : '静音'"
            :class="[circleBtnBase, circleBtnGhost]"
            @click="$emit('toggle-mute')"
          >
            <Icon :icon="volumeIcon" width="18" height="18" class="block shrink-0" aria-hidden="true" />
          </a-button>

          <div class="flex w-[140px] max-[720px]:hidden flex-col justify-center gap-0.5 pr-1">
            <div class="flex items-center justify-between text-[9px] leading-none uppercase tracking-[0.14em]" :class="isDark ? 'text-white/40' : 'text-slate-500'">
              <span>Master</span>
              <span>{{ formatPercent(muted ? 0 : volume) }}</span>
            </div>
            <div class="px-0.5">
              <a-slider
                :class="['w-full', masterSlider]"
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
            </div>
            <div class="flex items-center justify-between text-[9px] leading-none uppercase tracking-[0.12em]" :class="isDark ? 'text-white/32' : 'text-slate-400'">
              <span>L {{ leftChannelSummary }}</span>
              <span>{{ channelRoutingLabel(channelRouting) }}</span>
              <span>R {{ rightChannelSummary }}</span>
            </div>
          </div>
        </div>

        <span :class="divider" aria-hidden="true" />

        <a-button
          size="small"
          :disabled="disabled"
          :class="[tinyPillBtn, 'shrink-0', channelPanelOpen ? (isDark ? 'bg-white/14 text-white' : 'bg-black/8 text-slate-900') : '']"
          :title="channelPanelOpen ? '收起声道控制' : '展开声道控制'"
          @click="toggleChannelPanel"
        >
          <Icon icon="lucide:audio-lines" width="14" height="14" class="shrink-0" aria-hidden="true" />
          <span>L/R</span>
        </a-button>
      </div>

      <div
        v-if="channelPanelOpen"
        :class="channelPanel"
      >
        <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.18em]" :class="isDark ? 'text-white/50' : 'text-slate-500'">
          <span>Channel Trim</span>
          <span :class="isDark ? 'text-white/28' : 'text-slate-400'">{{ channelRoutingLabel(channelRouting) }}</span>
        </div>

        <div class="mb-3 rounded-xl border p-2.5" :class="isDark ? 'border-white/8 bg-white/3' : 'border-black/8 bg-black/3'">
          <div class="mb-2 flex items-center justify-between text-[11px]" :class="isDark ? 'text-white/78' : 'text-slate-700'">
            <div class="flex items-center gap-2">
              <span>Routing</span>
              <span class="text-[10px] uppercase tracking-[0.16em]" :class="isDark ? 'text-white/38' : 'text-slate-400'">Source Map</span>
            </div>
            <span class="text-[10px] uppercase tracking-[0.14em]" :class="isDark ? 'text-white/42' : 'text-slate-500'">
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
          <div class="rounded-xl border px-3 py-2.5" :class="isDark ? 'border-white/8 bg-white/3' : 'border-black/8 bg-black/3'">
            <div class="mb-2 flex items-center justify-between text-[11px]" :class="isDark ? 'text-white/78' : 'text-slate-700'">
              <div class="flex items-center gap-2">
                <span>L</span>
                <span class="text-[10px] uppercase tracking-[0.16em]" :class="isDark ? 'text-white/38' : 'text-slate-400'">Trim</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-[10px] uppercase tracking-[0.14em]" :class="isDark ? 'text-white/42' : 'text-slate-500'">
                  out {{ formatPercent(leftEffectiveOutput) }}
                </span>
                <a-button
                  size="small"
                  :disabled="disabled"
                  :class="[tinyPillBtn, 'h-7 px-2.5 text-[11px]']"
                  @click="$emit('set-left-channel-muted', !leftChannelMuted)"
                >
                  {{ leftChannelMuted ? "Unmute" : "Mute" }}
                </a-button>
              </div>
            </div>
            <div class="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.14em]" :class="isDark ? 'text-white/34' : 'text-slate-500'">
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

          <div class="rounded-xl border px-3 py-2.5" :class="isDark ? 'border-white/8 bg-white/3' : 'border-black/8 bg-black/3'">
            <div class="mb-2 flex items-center justify-between text-[11px]" :class="isDark ? 'text-white/78' : 'text-slate-700'">
              <div class="flex items-center gap-2">
                <span>R</span>
                <span class="text-[10px] uppercase tracking-[0.16em]" :class="isDark ? 'text-white/38' : 'text-slate-400'">Trim</span>
              </div>
              <div class="flex items-center gap-2.5">
                <span class="text-[10px] uppercase tracking-[0.14em]" :class="isDark ? 'text-white/42' : 'text-slate-500'">
                  out {{ formatPercent(rightEffectiveOutput) }}
                </span>
                <a-button
                  size="small"
                  :disabled="disabled"
                  :class="[tinyPillBtn, 'h-7 px-2.5 text-[11px]']"
                  @click="$emit('set-right-channel-muted', !rightChannelMuted)"
                >
                  {{ rightChannelMuted ? "Unmute" : "Mute" }}
                </a-button>
              </div>
            </div>
            <div class="mb-1 flex items-center justify-between text-[10px] uppercase tracking-[0.14em]" :class="isDark ? 'text-white/34' : 'text-slate-500'">
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
