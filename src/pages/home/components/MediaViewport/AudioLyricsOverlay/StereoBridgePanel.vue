<script setup lang="ts">
import type { MediaAudioMeterPayload } from "@/modules/media-types";
import AudioSpectrumChart from "../AudioSpectrumChart.vue";
import type { StereoBridgeChannel } from "./useAudioLyricPanel/types";

defineProps<{
  channels: StereoBridgeChannel[];
  audioMeter: MediaAudioMeterPayload | null;
  frameClass: string;
  captionClass: string;
  metaClass: string;
  compact: boolean;
  isDark?: boolean;
}>();
</script>

<template>
  <div :class="frameClass">
    <div class="mb-1 flex items-center justify-between text-[9px] uppercase tracking-[0.2em]" :class="captionClass">
      <span>Stereo</span>
      <span>{{ audioMeter?.channels ?? 0 }} ch</span>
    </div>
    <div class="grid gap-1 md:grid-cols-2">
      <div
        v-for="channel in channels"
        :key="channel.key"
        class="min-w-0 px-1"
      >
        <div class="mb-0.5 flex items-center justify-between text-[9px] uppercase tracking-[0.16em]" :class="metaClass">
          <span>{{ channel.label }}</span>
          <span class="truncate pl-2 text-[8px] normal-case tracking-normal">{{ channel.peakDbfs }}</span>
        </div>
        <AudioSpectrumChart
          :bars="channel.bars"
          :hold-bars="channel.holdBars"
          :peak-hold="channel.peakHold"
          :compact="compact"
          :is-dark="isDark"
        />
      </div>
    </div>
  </div>
</template>
