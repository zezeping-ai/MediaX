<script setup lang="ts">
import { useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import AudioSpectrumChart from "./AudioSpectrumChart.vue";
import type {
  MediaAudioMeterPayload,
  MediaLyricLine,
  PlaybackState,
} from "@/modules/media-types";

type CompactLyricEntry = {
  line: MediaLyricLine;
  absoluteIndex: number;
};

type StereoBridgeChannel = {
  key: "left" | "right";
  label: "L" | "R";
  bars: number[];
  holdBars: number[];
  peakHold: number;
  peakState: string;
  peakDbfs: string;
  holdDbfs: string;
};

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

const SPECTRUM_BAR_COUNT = 24;
const MIN_BAR_LEVEL = 0.04;
const HOLD_DECAY_STEP = 0.035;
const HOLD_TICK_MS = 70;

const interpolatedPosition = ref(0);
const leftHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
const rightHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
const leftPeakHold = ref(MIN_BAR_LEVEL);
const rightPeakHold = ref(MIN_BAR_LEVEL);

let lastTickAt = Date.now();
const timer = window.setInterval(() => {
  const playback = props.playback;
  if (!playback) {
    interpolatedPosition.value = 0;
    lastTickAt = Date.now();
    return;
  }
  if (playback.status !== "playing") {
    interpolatedPosition.value = playback.position_seconds;
    lastTickAt = Date.now();
    return;
  }
  const now = Date.now();
  const deltaSeconds = Math.max(0, now - lastTickAt) / 1000;
  lastTickAt = now;
  interpolatedPosition.value = Math.min(
    playback.duration_seconds || Number.MAX_SAFE_INTEGER,
    interpolatedPosition.value + deltaSeconds * playback.playback_rate,
  );
}, 80);
const holdTimer = window.setInterval(() => {
  leftHoldBars.value = leftHoldBars.value.map((value) => Math.max(MIN_BAR_LEVEL, value - HOLD_DECAY_STEP));
  rightHoldBars.value = rightHoldBars.value.map((value) => Math.max(MIN_BAR_LEVEL, value - HOLD_DECAY_STEP));
  leftPeakHold.value = Math.max(MIN_BAR_LEVEL, leftPeakHold.value - HOLD_DECAY_STEP);
  rightPeakHold.value = Math.max(MIN_BAR_LEVEL, rightPeakHold.value - HOLD_DECAY_STEP);
}, HOLD_TICK_MS);

watch(
  () => [
    props.playback?.position_seconds ?? 0,
    props.playback?.status ?? "idle",
    props.playback?.current_path ?? "",
  ],
  () => {
    interpolatedPosition.value = props.playback?.position_seconds ?? 0;
    lastTickAt = Date.now();
  },
  { immediate: true },
);

watch(
  () => props.audioMeter,
  (meter) => {
    const nextLeftBars = normalizeSpectrum(meter?.left_spectrum);
    const nextRightBars = normalizeSpectrum(meter?.right_spectrum);
    leftHoldBars.value = nextLeftBars.map((value, index) => Math.max(value, leftHoldBars.value[index] ?? MIN_BAR_LEVEL));
    rightHoldBars.value = nextRightBars.map((value, index) => Math.max(value, rightHoldBars.value[index] ?? MIN_BAR_LEVEL));
    leftPeakHold.value = Math.max(Math.max(MIN_BAR_LEVEL, meter?.left_peak ?? 0), leftPeakHold.value);
    rightPeakHold.value = Math.max(Math.max(MIN_BAR_LEVEL, meter?.right_peak ?? 0), rightPeakHold.value);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  window.clearInterval(timer);
  window.clearInterval(holdTimer);
});

const orderedLyrics = computed(() =>
  props.lyrics
    .filter((line) => Number.isFinite(line.time_seconds) && line.text.trim())
    .sort((a, b) => a.time_seconds - b.time_seconds),
);
const activeLyricIndex = computed(() => {
  const lyrics = orderedLyrics.value;
  if (lyrics.length === 0) {
    return -1;
  }
  const current = interpolatedPosition.value;
  for (let index = lyrics.length - 1; index >= 0; index -= 1) {
    if (current + 0.08 >= lyrics[index].time_seconds) {
      return index;
    }
  }
  return 0;
});
const visibleLyrics = computed<CompactLyricEntry[]>(() => {
  const lyrics = orderedLyrics.value;
  if (lyrics.length === 0) {
    return [];
  }
  const activeIndex = Math.max(activeLyricIndex.value, 0);
  const start = Math.max(0, activeIndex - 1);
  const end = Math.min(lyrics.length, activeIndex + 2);
  return lyrics.slice(start, end).map((line, offset) => ({
    line,
    absoluteIndex: start + offset,
  }));
});
const showAudioOverlay = computed(() => props.mediaKind === "audio");
const { height: viewportHeight } = useWindowSize();
const trackTitle = computed(() => props.title || "Unknown Track");
const trackSubtitle = computed(() =>
  [props.artist, props.album].filter(Boolean).join(" · "),
);
const playbackDurationSeconds = computed(() => Math.max(0, props.playback?.duration_seconds ?? 0));
const playbackPositionSeconds = computed(() => Math.max(0, interpolatedPosition.value));
const progressPercent = computed(() => {
  const duration = playbackDurationSeconds.value;
  if (!duration || !Number.isFinite(duration)) {
    return 0;
  }
  return Math.min(100, Math.max(0, (playbackPositionSeconds.value / duration) * 100));
});
const playbackStatusText = computed(() => {
  switch (props.playback?.status) {
    case "playing":
      return "Playing";
    case "paused":
      return "Paused";
    case "stopped":
      return "Stopped";
    default:
      return "Ready";
  }
});
const metadataChips = computed(() =>
  [
    props.artist ? `Artist · ${props.artist}` : "",
    props.album ? `Album · ${props.album}` : "",
    props.hasCoverArt ? "Cover Art" : "",
    props.audioMeter?.sample_rate ? `${props.audioMeter.sample_rate} Hz` : "",
  ].filter(Boolean),
);
const leftSpectrumBars = computed(() => normalizeSpectrum(props.audioMeter?.left_spectrum));
const rightSpectrumBars = computed(() => normalizeSpectrum(props.audioMeter?.right_spectrum));
const leftPeakLevel = computed(() => Math.max(0, Math.min(1, props.audioMeter?.left_peak ?? 0)));
const rightPeakLevel = computed(() => Math.max(0, Math.min(1, props.audioMeter?.right_peak ?? 0)));
const leftPeakDbfs = computed(() => formatDbfs(leftPeakLevel.value));
const rightPeakDbfs = computed(() => formatDbfs(rightPeakLevel.value));
const leftHoldDbfs = computed(() => formatDbfs(leftPeakHold.value));
const rightHoldDbfs = computed(() => formatDbfs(rightPeakHold.value));
const leftPeakState = computed(() => describePeakState(leftPeakLevel.value));
const rightPeakState = computed(() => describePeakState(rightPeakLevel.value));
const hasLyrics = computed(() => orderedLyrics.value.length > 0);
const isMasterMuted = computed(() => props.playback?.muted ?? false);
const showStereoBridge = computed(() => viewportHeight.value >= 700);
const useCompactStereoBridge = computed(() => viewportHeight.value < 900);
const useDenseStageLayout = computed(() => viewportHeight.value < 820);
const overlayShellClass = computed(() => (
  useDenseStageLayout.value
    ? "mx-auto flex h-full max-w-6xl flex-col gap-2 rounded-[24px] px-2 py-2 md:px-3 md:py-3"
    : "mx-auto flex h-full max-w-6xl flex-col gap-2.5 rounded-[28px] px-3 py-3 md:px-4 md:py-4"
));
const stereoBridgeChannels = computed<StereoBridgeChannel[]>(() => [
  {
    key: "left",
    label: "L",
    bars: leftSpectrumBars.value,
    holdBars: leftHoldBars.value,
    peakHold: leftPeakHold.value,
    peakState: leftPeakState.value,
    peakDbfs: leftPeakDbfs.value,
    holdDbfs: leftHoldDbfs.value,
  },
  {
    key: "right",
    label: "R",
    bars: rightSpectrumBars.value,
    holdBars: rightHoldBars.value,
    peakHold: rightPeakHold.value,
    peakState: rightPeakState.value,
    peakDbfs: rightPeakDbfs.value,
    holdDbfs: rightHoldDbfs.value,
  },
]);
const stereoBridgeFrameClass = computed(() => (
  useDenseStageLayout.value
    ? "rounded-lg border border-white/8 px-2.5 py-2.25"
    : "rounded-xl border border-white/8 px-3 py-2.75"
));
const stageFrameClass = computed(() => (
  useDenseStageLayout.value
    ? "rounded-xl border border-white/8 px-3 py-2.5"
    : "rounded-2xl border border-white/8 px-4 py-3 md:px-5 md:py-4"
));
const titleBlockClass = computed(() => (useDenseStageLayout.value ? "mt-2" : "mt-3.5"));
const metadataRowClass = computed(() => (
  useDenseStageLayout.value ? "mt-2 flex flex-wrap gap-1.5" : "mt-2.5 flex flex-wrap gap-1.5"
));
const bodySectionClass = computed(() => (
  useDenseStageLayout.value ? "mt-1.5 pt-0.5" : "mt-2.5 pt-1"
));
const lyricsViewportClass = computed(() => (
  useDenseStageLayout.value
    ? "relative flex h-24 flex-col justify-center overflow-hidden md:h-28"
    : "relative flex h-32 flex-col justify-center overflow-hidden md:h-36"
));
const titleTextClass = computed(() => (
  useDenseStageLayout.value
    ? "text-xl font-semibold tracking-[0.03em] text-white/96 md:text-3xl"
    : "text-2xl font-semibold tracking-[0.03em] text-white/96 md:text-4xl"
));
const footerFrameClass = computed(() => (
  useDenseStageLayout.value
    ? "flex h-[5rem] flex-col border border-white/8 px-3 py-2"
    : "flex h-[5.25rem] flex-col border border-white/8 px-4 py-2"
));
const emptyStateLabel = computed(() => (props.hasCoverArt ? "Cover View" : "Metadata View"));
function normalizeSpectrum(values?: number[] | null) {
  const input = Array.isArray(values) ? values : [];
  return Array.from({ length: SPECTRUM_BAR_COUNT }, (_, index) => {
    const raw = input[index] ?? 0;
    return Math.max(MIN_BAR_LEVEL, Math.min(1, raw));
  });
}

function formatClock(totalSeconds: number) {
  if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) {
    return "00:00";
  }
  const rounded = Math.floor(totalSeconds);
  const hours = Math.floor(rounded / 3600);
  const minutes = Math.floor((rounded % 3600) / 60);
  const seconds = rounded % 60;
  if (hours > 0) {
    return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function formatDbfs(level: number) {
  const normalized = Math.max(0, Math.min(1, level));
  if (normalized <= 0.000_01) {
    return "-inf dBFS";
  }
  const db = 20 * Math.log10(normalized);
  if (db >= -0.05) {
    return "0.0 dBFS";
  }
  return `${db.toFixed(1)} dBFS`;
}

function describePeakState(level: number) {
  const normalized = Math.max(0, Math.min(1, level));
  if (normalized >= 0.995) {
    return "CLIP";
  }
  if (normalized >= 0.89) {
    return "HOT";
  }
  if (normalized >= 0.5) {
    return "LIVE";
  }
  return "CALM";
}

</script>

<template>
  <div
    v-if="showAudioOverlay"
    class="absolute inset-0 z-20 overflow-hidden"
  >
    <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.06),rgba(0,0,0,0.60))]" />
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
              <div>
                <AudioSpectrumChart
                  :bars="channel.bars"
                  :hold-bars="channel.holdBars"
                  :peak-hold="channel.peakHold"
                  :compact="useCompactStereoBridge"
                />
              </div>
            </div>
          </div>
        </div>

        <div class="min-h-0">
          <div :class="stageFrameClass">
            <div class="flex flex-wrap items-center gap-2">
              <span class="border border-white/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.22em] text-white/70">
                {{ playbackStatusText }}
              </span>
              <span class="text-[10px] uppercase tracking-[0.24em] text-white/38">Audio Stage</span>
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
                <div class="pointer-events-none absolute inset-x-0 top-0 h-10 bg-gradient-to-b from-black/40 to-transparent" />
                <div class="pointer-events-none absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-black/40 to-transparent" />
                <div class="relative py-2">
                  <p
                    v-for="entry in visibleLyrics"
                    :key="`${entry.line.time_seconds}-${entry.absoluteIndex}`"
                    class="py-1.5 text-[14px] tracking-[0.05em] text-white/24 transition-all duration-300 md:text-base"
                    :class="entry.absoluteIndex === activeLyricIndex ? 'scale-[1.01] text-lg text-white/98 [text-shadow:0_4px_24px_rgba(0,0,0,0.5)] md:text-[20px]' : ''"
                  >
                    {{ entry.line.text }}
                  </p>
                </div>
              </div>
            </div>

            <div
              v-else
              :class="bodySectionClass"
            >
              <div class="flex items-center justify-between text-[10px] uppercase tracking-[0.22em] text-white/34">
                <span>Lyrics</span>
                <span>{{ emptyStateLabel }}</span>
              </div>
            </div>
          </div>
        </div>
        <div class="shrink-0">
          <div class="mx-auto max-w-5xl">
            <div :class="footerFrameClass">
              <div class="mb-1 flex items-center justify-between text-[11px] tracking-[0.18em] text-white/52">
                <span>{{ formatClock(playbackPositionSeconds) }}</span>
                <span>{{ formatClock(playbackDurationSeconds) }}</span>
              </div>
              <div class="mt-1 h-1.5 shrink-0 overflow-hidden rounded-full bg-white/10">
                <div
                  class="h-full rounded-full bg-[linear-gradient(90deg,rgba(255,255,255,0.72),rgba(255,255,255,0.34))] transition-[width] duration-150 ease-out"
                  :style="{ width: `${progressPercent}%` }"
                />
              </div>
              <div class="mt-auto pt-1.5 flex items-center justify-between text-[10px] uppercase tracking-[0.22em] text-white/34">
                <span>{{ hasLyrics ? "Lyrics Sync" : "Metadata View" }}</span>
                <span>{{ playbackStatusText }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
