<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import AudioSpectrumChart from "./AudioSpectrumChart.vue";
import type {
  MediaAudioMeterPayload,
  MediaLyricLine,
  PlaybackState,
} from "@/modules/media-types";

const props = defineProps<{
  mediaKind: "video" | "audio";
  playback: PlaybackState | null;
  audioMeter: MediaAudioMeterPayload | null;
  lyrics: MediaLyricLine[];
  title: string;
  artist: string;
  album: string;
  hasCoverArt: boolean;
  setLeftChannelVolume: (volume: number) => Promise<void>;
  setRightChannelVolume: (volume: number) => Promise<void>;
  setLeftChannelMuted: (muted: boolean) => Promise<void>;
  setRightChannelMuted: (muted: boolean) => Promise<void>;
}>();

const SPECTRUM_BAR_COUNT = 24;
const MIN_BAR_LEVEL = 0.04;
const HOLD_DECAY_STEP = 0.035;
const HOLD_TICK_MS = 70;

const interpolatedPosition = ref(0);
const leftVolume = ref(1);
const rightVolume = ref(1);
const leftMuted = ref(false);
const rightMuted = ref(false);
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
  () => props.playback,
  (playback) => {
    leftVolume.value = playback?.left_channel_volume ?? 1;
    rightVolume.value = playback?.right_channel_volume ?? 1;
    leftMuted.value = playback?.left_channel_muted ?? false;
    rightMuted.value = playback?.right_channel_muted ?? false;
  },
  { immediate: true, deep: true },
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

const normalizedLyrics = computed(() =>
  props.lyrics
    .filter((line) => Number.isFinite(line.time_seconds) && line.text.trim())
    .sort((a, b) => a.time_seconds - b.time_seconds),
);
const compactLyrics = computed(() => {
  const lyrics = normalizedLyrics.value;
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

const activeLyricIndex = computed(() => {
  const lyrics = normalizedLyrics.value;
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

const shouldShow = computed(() => props.mediaKind === "audio");
const fallbackHeadline = computed(() => props.title || "Unknown Track");
const fallbackSubline = computed(() =>
  [props.artist, props.album].filter(Boolean).join(" · "),
);
const safeDuration = computed(() => Math.max(0, props.playback?.duration_seconds ?? 0));
const safePosition = computed(() => Math.max(0, interpolatedPosition.value));
const progressPercent = computed(() => {
  const duration = safeDuration.value;
  if (!duration || !Number.isFinite(duration)) {
    return 0;
  }
  return Math.min(100, Math.max(0, (safePosition.value / duration) * 100));
});
const playbackStatusLabel = computed(() => {
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
const metaChips = computed(() =>
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
const hasLyrics = computed(() => normalizedLyrics.value.length > 0);
const globalMuted = computed(() => props.playback?.muted ?? false);

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

async function commitLeftVolume(value: number | number[]) {
  const normalized = Array.isArray(value) ? value[0] : value;
  leftVolume.value = normalized;
  leftMuted.value = normalized <= 0;
  await props.setLeftChannelVolume(normalized);
}

async function commitRightVolume(value: number | number[]) {
  const normalized = Array.isArray(value) ? value[0] : value;
  rightVolume.value = normalized;
  rightMuted.value = normalized <= 0;
  await props.setRightChannelVolume(normalized);
}

async function toggleLeftMuted() {
  leftMuted.value = !leftMuted.value;
  await props.setLeftChannelMuted(leftMuted.value);
}

async function toggleRightMuted() {
  rightMuted.value = !rightMuted.value;
  await props.setRightChannelMuted(rightMuted.value);
}
</script>

<template>
  <div
    v-if="shouldShow"
    class="absolute inset-0 z-20 overflow-hidden"
  >
    <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(255,255,255,0.06),rgba(0,0,0,0.60))]" />
    <div class="absolute inset-x-5 top-5 md:left-7 md:right-7 md:top-6">
      <div class="mx-auto max-w-6xl border-y border-white/12 bg-black/26 px-3 py-3">
        <div class="grid gap-0 lg:grid-cols-[minmax(0,1fr)_480px]">
          <div class="min-w-0 px-2 py-1 lg:pr-6">
            <div class="flex flex-wrap items-center gap-2">
            <span class="border border-white/12 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.22em] text-white/72">
              {{ playbackStatusLabel }}
            </span>
            <span class="text-[10px] uppercase tracking-[0.24em] text-white/44">Audio Session</span>
            <span
              v-if="globalMuted"
              class="border border-red-300/20 px-2 py-0.5 text-[10px] uppercase tracking-[0.2em] text-red-100/80"
            >
              Master Muted
            </span>
            </div>
            <div class="mt-2 flex items-end justify-between gap-4">
              <div class="min-w-0">
                <p class="truncate text-xl font-semibold tracking-[0.03em] text-white/95 md:text-2xl">
                  {{ fallbackHeadline }}
                </p>
                <p v-if="fallbackSubline" class="mt-1 truncate text-sm tracking-[0.08em] text-white/58 md:text-[15px]">
                  {{ fallbackSubline }}
                </p>
              </div>
              <div class="hidden text-[10px] uppercase tracking-[0.18em] text-white/36 lg:block">
                {{ props.audioMeter?.channels ?? 0 }} ch
              </div>
            </div>
            <div v-if="metaChips.length > 0" class="mt-3 flex flex-wrap gap-2">
              <span
                v-for="chip in metaChips"
                :key="chip"
                class="border border-white/10 px-2.5 py-1 text-[10px] tracking-[0.12em] text-white/60"
              >
                {{ chip }}
              </span>
            </div>
            <div
              v-if="hasLyrics"
              class="mt-4 border-t border-white/10 pt-3"
            >
              <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.2em] text-white/36">
                <span>Lyrics</span>
                <span>{{ activeLyricIndex >= 0 ? `${activeLyricIndex + 1}/${normalizedLyrics.length}` : "Ready" }}</span>
              </div>
              <div class="relative flex h-28 flex-col justify-center overflow-hidden">
                <div class="pointer-events-none absolute inset-x-0 top-0 h-8 bg-gradient-to-b from-black/40 to-transparent" />
                <div class="pointer-events-none absolute inset-x-0 bottom-0 h-8 bg-gradient-to-t from-black/40 to-transparent" />
                <div class="relative py-1">
                  <p
                    v-for="entry in compactLyrics"
                    :key="`${entry.line.time_seconds}-${entry.absoluteIndex}`"
                    class="py-1 text-left text-[13px] tracking-[0.04em] text-white/18 transition-all duration-300 md:text-sm"
                    :class="entry.absoluteIndex === activeLyricIndex ? 'scale-[1.01] text-base text-white/96 md:text-[15px]' : ''"
                  >
                    {{ entry.line.text }}
                  </p>
                </div>
              </div>
            </div>
            <div
              v-else
              class="mt-4 border-t border-white/10 pt-3"
            >
              <p v-if="fallbackSubline" class="text-sm tracking-[0.12em] text-white/52 md:text-[15px]">
                {{ fallbackSubline }}
              </p>
              <p class="mt-2 max-w-2xl text-xs leading-6 tracking-[0.08em] text-white/34 md:text-sm">
                {{ hasCoverArt ? "Embedded cover art is rendering behind the audio stage, while the live stereo spectrum stays in the top channel strip." : "Pure audio playback mode is active, with the live stereo spectrum embedded in the top channel strip." }}
              </p>
            </div>
          </div>

          <div class="pointer-events-auto border-t border-white/10 px-2 py-2 lg:border-l lg:border-t-0 lg:pl-6">
            <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.22em] text-white/42">
              <span>Channel Mixer</span>
              <span>{{ props.audioMeter?.channels ?? 0 }} ch</span>
            </div>
            <div class="grid gap-4 md:grid-cols-2">
              <div class="pt-1 md:border-r md:border-white/10 md:pr-4">
                <div class="mb-2 flex items-center justify-between text-[11px] uppercase tracking-[0.2em] text-white/56">
                <span>Left</span>
                <span class="text-[10px] tracking-[0.16em] text-white/28">Channel A</span>
                </div>
                <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-white/44">
                <span>{{ leftPeakState }}</span>
                <span>{{ leftPeakDbfs }}</span>
                </div>
                <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-white/28">
                <span>Hold</span>
                <span>{{ leftHoldDbfs }}</span>
                </div>
                <div class="mb-3 border-y border-white/8">
                  <AudioSpectrumChart
                    :bars="leftSpectrumBars"
                    :hold-bars="leftHoldBars"
                    :peak-hold="leftPeakHold"
                  />
                </div>
                <div class="flex items-center gap-3">
                  <button
                    class="flex h-8 min-w-8 items-center justify-center border border-white/10 px-2 text-[10px] uppercase tracking-[0.16em] text-white/70 transition-colors hover:border-white/22 hover:text-white/88"
                    @click="void toggleLeftMuted()"
                  >
                    {{ leftMuted ? "Off" : "On" }}
                  </button>
                  <a-slider
                    class="flex-1"
                    :value="leftMuted ? 0 : leftVolume"
                    :min="0"
                    :max="1"
                    :step="0.01"
                    :tooltip-open="false"
                    @change="(value) => void commitLeftVolume(value)"
                  />
                </div>
              </div>

              <div class="border-t border-white/10 pt-3 md:border-t-0 md:pl-1 md:pt-1">
                <div class="mb-2 flex items-center justify-between text-[11px] uppercase tracking-[0.2em] text-white/56">
                <span>Right</span>
                <span class="text-[10px] tracking-[0.16em] text-white/28">Channel B</span>
                </div>
                <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.18em] text-white/44">
                <span>{{ rightPeakState }}</span>
                <span>{{ rightPeakDbfs }}</span>
                </div>
                <div class="mb-2 flex items-center justify-between text-[10px] uppercase tracking-[0.14em] text-white/28">
                <span>Hold</span>
                <span>{{ rightHoldDbfs }}</span>
                </div>
                <div class="mb-3 border-y border-white/8">
                  <AudioSpectrumChart
                    :bars="rightSpectrumBars"
                    :hold-bars="rightHoldBars"
                    :peak-hold="rightPeakHold"
                  />
                </div>
                <div class="flex items-center gap-3">
                  <button
                    class="flex h-8 min-w-8 items-center justify-center border border-white/10 px-2 text-[10px] uppercase tracking-[0.16em] text-white/70 transition-colors hover:border-white/22 hover:text-white/88"
                    @click="void toggleRightMuted()"
                  >
                    {{ rightMuted ? "Off" : "On" }}
                  </button>
                  <a-slider
                    class="flex-1"
                    :value="rightMuted ? 0 : rightVolume"
                    :min="0"
                    :max="1"
                    :step="0.01"
                    :tooltip-open="false"
                    @change="(value) => void commitRightVolume(value)"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="absolute inset-x-5 bottom-5 md:left-7 md:right-7 md:bottom-6">
      <div class="mx-auto max-w-6xl">
        <div class="border border-white/12 bg-black/22 px-4 py-3">
          <div class="mb-2 flex items-center justify-between text-[11px] tracking-[0.18em] text-white/52">
            <span>{{ formatClock(safePosition) }}</span>
            <span>{{ formatClock(safeDuration) }}</span>
          </div>
          <div class="h-1.5 overflow-hidden rounded-full bg-white/10">
            <div
              class="h-full rounded-full bg-[linear-gradient(90deg,rgba(255,255,255,0.72),rgba(255,255,255,0.34))] transition-[width] duration-150 ease-out"
              :style="{ width: `${progressPercent}%` }"
            />
          </div>
          <div class="mt-2 flex items-center justify-between text-[10px] uppercase tracking-[0.22em] text-white/38">
            <span>Top Stereo Spectrum</span>
            <span>{{ hasLyrics ? "Lyrics Sync" : "Metadata View" }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
