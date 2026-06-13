import { computed, ref, watch, type Ref } from "vue";
import type { MediaAudioMeterPayload } from "@/modules/media-types";
import type { StereoBridgeChannel } from "./types";

const SPECTRUM_BAR_COUNT = 24;
const MIN_BAR_LEVEL = 0.06;
const HOLD_DECAY_STEP = 0.035;
const HOLD_TICK_MS = 70;

type UseStereoBridgeOptions = {
  audioMeter: Readonly<Ref<MediaAudioMeterPayload | null>>;
  onDecayTick?: (elapsedMs: number) => void;
};

export function useStereoBridge(options: UseStereoBridgeOptions) {
  const leftHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const rightHoldBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const leftSpectrumBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const rightSpectrumBars = ref(Array.from({ length: SPECTRUM_BAR_COUNT }, () => MIN_BAR_LEVEL));
  const leftPeakHold = ref(MIN_BAR_LEVEL);
  const rightPeakHold = ref(MIN_BAR_LEVEL);

  function decay(elapsedMs: number) {
    const decaySteps = Math.max(1, Math.floor(elapsedMs / HOLD_TICK_MS));
    decayBarsInPlace(leftHoldBars.value, decaySteps);
    decayBarsInPlace(rightHoldBars.value, decaySteps);
    leftPeakHold.value = Math.max(MIN_BAR_LEVEL, leftPeakHold.value - HOLD_DECAY_STEP * decaySteps);
    rightPeakHold.value = Math.max(MIN_BAR_LEVEL, rightPeakHold.value - HOLD_DECAY_STEP * decaySteps);
  }

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

  const stereoBridgeChannels = computed<StereoBridgeChannel[]>(() => [
    buildChannel("left", "L", leftSpectrumBars.value, leftHoldBars.value, leftPeakHold.value, options.audioMeter.value?.left_peak ?? 0),
    buildChannel("right", "R", rightSpectrumBars.value, rightHoldBars.value, rightPeakHold.value, options.audioMeter.value?.right_peak ?? 0),
  ]);

  return {
    decay,
    stereoBridgeChannels,
  };
}

function buildChannel(
  key: "left" | "right",
  label: "L" | "R",
  bars: number[],
  holdBars: number[],
  peakHold: number,
  peak: number,
): StereoBridgeChannel {
  const normalizedPeak = Math.max(0, Math.min(1, peak));
  return {
    key,
    label,
    bars,
    holdBars,
    peakHold,
    peakState: describePeakState(normalizedPeak),
    peakDbfs: formatDbfs(normalizedPeak),
    holdDbfs: formatDbfs(peakHold),
  };
}

function normalizeSpectrumInto(target: number[], values?: number[] | null) {
  const input = Array.isArray(values) ? values : [];
  for (let index = 0; index < SPECTRUM_BAR_COUNT; index += 1) {
    const raw = input[index] ?? 0;
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
  if (level >= 0.995) {
    return "CLIP";
  }
  if (level >= 0.89) {
    return "HOT";
  }
  if (level >= 0.5) {
    return "LIVE";
  }
  return "CALM";
}
