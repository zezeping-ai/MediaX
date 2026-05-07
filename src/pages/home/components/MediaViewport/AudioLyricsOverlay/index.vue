<script setup lang="ts">
import { toRef } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaLyricLine,
  PlaybackState,
} from "@/modules/media-types";
import AudioSpectrumChart from "../AudioSpectrumChart.vue";
import { useAudioLyricPanel } from "./useAudioLyricsOverlay";

const props = defineProps<{
  mediaKind: "video" | "audio";
  playback: PlaybackState | null;
  audioMeter: MediaAudioMeterPayload | null;
  lyrics: MediaLyricLine[];
  title: string;
  artist: string;
  album: string;
  hasCoverArt: boolean;
}>();

const {
  activeLyricIndex,
  bodySectionClass,
  hasLyrics,
  isMasterMuted,
  lyricsViewportClass,
  metadataChips,
  metadataRowClass,
  orderedLyrics,
  overlayShellClass,
  playbackStatusText,
  showAudioLyricPanel,
  showStereoBridge,
  stageFrameClass,
  stereoBridgeChannels,
  stereoBridgeFrameClass,
  titleBlockClass,
  titleTextClass,
  trackSubtitle,
  trackTitle,
  useCompactStereoBridge,
  visibleLyrics,
} = useAudioLyricPanel({
  mediaKind: toRef(props, "mediaKind"),
  playback: toRef(props, "playback"),
  audioMeter: toRef(props, "audioMeter"),
  lyrics: toRef(props, "lyrics"),
  title: toRef(props, "title"),
  artist: toRef(props, "artist"),
  album: toRef(props, "album"),
  hasCoverArt: toRef(props, "hasCoverArt"),
});
</script>

<template>
  <div
    v-if="showAudioLyricPanel"
    class="pointer-events-none absolute inset-0 z-20 overflow-hidden"
  >
    <div
      class="pointer-events-none absolute inset-0"
      :class="props.hasCoverArt
        ? 'bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.03),rgba(0,0,0,0.24))]'
        : 'bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.06),rgba(0,0,0,0.60))]'"
    />
    <div class="absolute inset-x-5 bottom-5 top-4 md:bottom-6 md:left-7 md:right-7 md:top-5">
      <div :class="overlayShellClass">
        <div
          v-if="showStereoBridge"
          :class="stereoBridgeFrameClass"
        >
          <div class="mb-1.5 flex items-center justify-between text-[10px] uppercase tracking-[0.22em] text-white/38">
            <span>Stereo Bridge</span>
            <span>{{ props.audioMeter?.channels ?? 0 }} ch · {{ useCompactStereoBridge ? "Compact" : "Live Meter" }}</span>
          </div>
          <div class="grid gap-1.5 md:grid-cols-2">
            <div
              v-for="channel in stereoBridgeChannels"
              :key="channel.key"
              class="min-w-0 px-1.5"
            >
              <div class="mb-0.5 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-white/52">
                <span>{{ channel.label }}</span>
                <span class="truncate pl-3">{{ channel.peakState }} · {{ channel.peakDbfs }} · Hold {{ channel.holdDbfs }}</span>
              </div>
              <AudioSpectrumChart
                :bars="channel.bars"
                :hold-bars="channel.holdBars"
                :peak-hold="channel.peakHold"
                :compact="useCompactStereoBridge"
              />
            </div>
          </div>
        </div>

        <div class="min-h-0">
          <div :class="stageFrameClass">
            <div class="flex flex-wrap items-center gap-2">
              <span class="border border-white/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.22em] text-white/70">
                {{ playbackStatusText }}
              </span>
              <span class="text-[10px] uppercase tracking-[0.24em] text-white/38">歌词面板</span>
              <span
                v-if="isMasterMuted"
                class="border border-red-300/20 px-2 py-0.5 text-[10px] uppercase tracking-[0.2em] text-red-100/80"
              >
                Master Muted
              </span>
            </div>

            <div :class="titleBlockClass">
              <p :class="titleTextClass">
                {{ trackTitle }}
              </p>
              <p
                v-if="trackSubtitle"
                class="mt-1.5 text-sm tracking-[0.12em] text-white/58 md:text-base"
              >
                {{ trackSubtitle }}
              </p>
            </div>

            <div v-if="metadataChips.length > 0" :class="metadataRowClass">
              <span
                v-for="chip in metadataChips"
                :key="chip"
                class="rounded-full border border-white/8 px-2.5 py-1 text-[10px] tracking-[0.12em] text-white/62"
              >
                {{ chip }}
              </span>
            </div>

            <div
              v-if="hasLyrics"
              :class="bodySectionClass"
            >
              <div class="mb-1.5 flex items-center justify-between text-[10px] uppercase tracking-[0.2em] text-white/40">
                <span>Lyrics</span>
                <span>{{ activeLyricIndex >= 0 ? `${activeLyricIndex + 1}/${orderedLyrics.length}` : "Ready" }}</span>
              </div>
              <div :class="lyricsViewportClass">
                <div class="pointer-events-none absolute inset-x-0 top-0 h-10 bg-linear-to-b from-black/40 to-transparent" />
                <div class="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-linear-to-t from-black/40 to-transparent" />
                <div class="relative py-2">
                  <p
                    v-for="entry in visibleLyrics"
                    :key="`${entry.line.time_seconds}-${entry.absoluteIndex}`"
                    class="py-1.5 text-[14px] tracking-wider text-white/24 transition-all duration-300 md:text-base"
                    :class="entry.absoluteIndex === activeLyricIndex ? 'scale-[1.01] text-lg text-white/98 [text-shadow:0_4px_24px_rgba(0,0,0,0.5)] md:text-[20px]' : ''"
                  >
                    {{ entry.line.text }}
                  </p>
                </div>
              </div>
            </div>

            <div v-else :class="bodySectionClass" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
