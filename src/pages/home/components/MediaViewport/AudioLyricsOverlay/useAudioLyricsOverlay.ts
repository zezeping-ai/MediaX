import { useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch, type Ref } from "vue";
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

type UseAudioLyricsOverlayOptions = {
  mediaKind: Readonly<Ref<"video" | "audio">>;
  playback: Readonly<Ref<PlaybackState | null>>;
  audioMeter: Readonly<Ref<MediaAudioMeterPayload | null>>;
  lyrics: Readonly<Ref<MediaLyricLine[]>>;
  title: Readonly<Ref<string>>;
  artist: Readonly<Ref<string>>;
  album: Readonly<Ref<string>>;
  hasCoverArt: Readonly<Ref<boolean>>;
};

const SPECTRUM_BAR_COUNT = 24;
const MIN_BAR_LEVEL = 0.06;
const HOLD_DECAY_STEP = 0.035;
const HOLD_TICK_MS = 70;
const OVERLAY_ACTIVITY_TICK_MS = 100;

export function useAudioLyricsOverlay(options: UseAudioLyricsOverlayOptions) {
  const interpolatedPosition = ref(0);
  const leftHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const rightHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const leftSpectrumBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const rightSpectrumBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const leftPeakHold = ref(MIN_BAR_LEVEL);
  const rightPeakHold = ref(MIN_BAR_LEVEL);
  const showAudioOverlay = computed(() => options.mediaKind.value === "audio");

  let activityTimer: number | null = null;
  let lastTickAt = Date.now();

  function stopActivityTicker() {
    if (activityTimer === null) {
      return;
    }
    window.clearTimeout(activityTimer);
    activityTimer = null;
  }

  function scheduleActivityTick() {
    if (activityTimer !== null) {
      return;
    }
    activityTimer = window.setTimeout(() => {
      activityTimer = null;
      runActivityTick();
    }, OVERLAY_ACTIVITY_TICK_MS);
  }

  function runActivityTick() {
    const playback = options.playback.value;
    const now = Date.now();
    const elapsedMs = Math.max(0, now - lastTickAt);
    lastTickAt = now;
    if (!showAudioOverlay.value || !playback || playback.status !== "playing") {
      interpolatedPosition.value = playback?.position_seconds ?? 0;
      stopActivityTicker();
      return;
    }
    const durationSeconds = playback.duration_seconds || Number.MAX_SAFE_INTEGER;
    interpolatedPosition.value = Math.min(
      durationSeconds,
      interpolatedPosition.value + (elapsedMs / 1000) * playback.playback_rate,
    );
    const decaySteps = Math.max(1, Math.floor(elapsedMs / HOLD_TICK_MS));
    decayBarsInPlace(leftHoldBars.value, decaySteps);
    decayBarsInPlace(rightHoldBars.value, decaySteps);
    leftPeakHold.value = Math.max(MIN_BAR_LEVEL, leftPeakHold.value - HOLD_DECAY_STEP * decaySteps);
    rightPeakHold.value = Math.max(MIN_BAR_LEVEL, rightPeakHold.value - HOLD_DECAY_STEP * decaySteps);
    scheduleActivityTick();
  }

  function syncActivityTicker() {
    lastTickAt = Date.now();
    if (showAudioOverlay.value && options.playback.value?.status === "playing") {
      scheduleActivityTick();
      return;
    }
    stopActivityTicker();
  }

  watch(
    () => [
      showAudioOverlay.value,
      options.playback.value?.position_seconds ?? 0,
      options.playback.value?.status ?? "idle",
      options.playback.value?.playback_rate ?? 1,
      options.playback.value?.current_path ?? "",
    ],
    () => {
      interpolatedPosition.value = options.playback.value?.position_seconds ?? 0;
      syncActivityTicker();
    },
    { immediate: true },
  );

  watch(
    options.audioMeter,
    (meter) => {
      normalizeSpectrumInto(leftSpectrumBars.value, meter?.left_spectrum);
      normalizeSpectrumInto(rightSpectrumBars.value, meter?.right_spectrum);
      mergeHoldBarsInPlace(leftHoldBars.value, leftSpectrumBars.value);
      mergeHoldBarsInPlace(rightHoldBars.value, rightSpectrumBars.value);
      leftPeakHold.value = Math.max(Math.max(MIN_BAR_LEVEL, meter?.left_peak ?? 0), leftPeakHold.value);
      rightPeakHold.value = Math.max(Math.max(MIN_BAR_LEVEL, meter?.right_peak ?? 0), rightPeakHold.value);
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    stopActivityTicker();
  });

  const orderedLyrics = computed(() =>
    options.lyrics.value
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
  const { height: viewportHeight } = useWindowSize();
  const trackTitle = computed(() => options.title.value || "Unknown Track");
  const trackSubtitle = computed(() =>
    [options.artist.value, options.album.value].filter(Boolean).join(" · "),
  );
  const playbackDurationSeconds = computed(() => Math.max(0, options.playback.value?.duration_seconds ?? 0));
  const playbackPositionSeconds = computed(() => Math.max(0, interpolatedPosition.value));
  const progressPercent = computed(() => {
    const duration = playbackDurationSeconds.value;
    if (!duration || !Number.isFinite(duration)) {
      return 0;
    }
    return Math.min(100, Math.max(0, (playbackPositionSeconds.value / duration) * 100));
  });
  const playbackStatusText = computed(() => {
    switch (options.playback.value?.status) {
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
      options.artist.value ? `Artist · ${options.artist.value}` : "",
      options.album.value ? `Album · ${options.album.value}` : "",
      options.hasCoverArt.value ? "Cover Art" : "",
      options.audioMeter.value?.sample_rate ? `${options.audioMeter.value.sample_rate} Hz` : "",
    ].filter(Boolean),
  );
  const leftPeakLevel = computed(() => Math.max(0, Math.min(1, options.audioMeter.value?.left_peak ?? 0)));
  const rightPeakLevel = computed(() => Math.max(0, Math.min(1, options.audioMeter.value?.right_peak ?? 0)));
  const leftPeakDbfs = computed(() => formatDbfs(leftPeakLevel.value));
  const rightPeakDbfs = computed(() => formatDbfs(rightPeakLevel.value));
  const leftHoldDbfs = computed(() => formatDbfs(leftPeakHold.value));
  const rightHoldDbfs = computed(() => formatDbfs(rightPeakHold.value));
  const leftPeakState = computed(() => describePeakState(leftPeakLevel.value));
  const rightPeakState = computed(() => describePeakState(rightPeakLevel.value));
  const hasLyrics = computed(() => orderedLyrics.value.length > 0);
  const isMasterMuted = computed(() => options.playback.value?.muted ?? false);
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
  const emptyStateLabel = computed(() => (options.hasCoverArt.value ? "Cover View" : "Metadata View"));

  return {
    activeLyricIndex,
    emptyStateLabel,
    footerFrameClass,
    formatClock,
    hasLyrics,
    isMasterMuted,
    metadataChips,
    orderedLyrics,
    overlayShellClass,
    playbackDurationSeconds,
    playbackPositionSeconds,
    playbackStatusText,
    progressPercent,
    showAudioOverlay,
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
    bodySectionClass,
    lyricsViewportClass,
    metadataRowClass,
  };
}

function normalizeSpectrumInto(target: number[], values?: number[] | null) {
  const input = Array.isArray(values) ? values : [];
  for (let index = 0; index < SPECTRUM_BAR_COUNT; index += 1) {
    const raw = input[index] ?? 0;
    // Slightly lift low-level energy so spectrum bars appear earlier on quiet passages.
    const lifted = Math.pow(Math.max(0, Math.min(1, raw)), 0.72);
    target[index] = Math.max(MIN_BAR_LEVEL, Math.min(1, lifted));
  }
}

function mergeHoldBarsInPlace(target: number[], source: number[]) {
  for (let index = 0; index < SPECTRUM_BAR_COUNT; index += 1) {
    target[index] = Math.max(source[index] ?? MIN_BAR_LEVEL, target[index] ?? MIN_BAR_LEVEL);
  }
}

function decayBarsInPlace(target: number[], steps = 1) {
  const decayAmount = HOLD_DECAY_STEP * Math.max(1, steps);
  for (let index = 0; index < SPECTRUM_BAR_COUNT; index += 1) {
    target[index] = Math.max(MIN_BAR_LEVEL, target[index] - decayAmount);
  }
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
