import { computed, ref, watch } from "vue";
import { useLocalStorage } from "@vueuse/core";
import { clamp } from "lodash-es";
import type { PlaybackQualityMode } from "@/modules/media-types";
import { QUALITY_DOWNGRADE_LEVELS } from "../components/PlaybackControls/playbackControls.constants";
import { buildPlaybackQualityOptions } from "../components/PlaybackControls/playbackControlsUtils";
import { useMediaCenter } from "./useMediaCenter";
import { usePlaybackShortcuts } from "./usePlaybackShortcuts";
import { usePlayerOverlayControls } from "./usePlayerOverlayControls";

const QUALITY_BASELINE_STORAGE_KEY = "mediax:quality-baseline-by-source";

export function useHomePlaybackController() {
  const sourceHeightBaselineByPath = useLocalStorage<Record<string, number>>(
    QUALITY_BASELINE_STORAGE_KEY,
    {},
  );
  const mediaCenter = useMediaCenter();
  const playbackRate = ref(1);
  const volume = ref(1);
  const muted = ref(false);
  const selectedQuality = ref("source");
  const sourceVideoHeightBaseline = ref<number | null>(null);
  const playerErrorMessage = ref("");

  const displayErrorMessage = computed(
    () => playerErrorMessage.value || mediaCenter.errorMessage.value,
  );
  const hasSource = computed(() => Boolean(mediaCenter.currentSource.value));
  const adaptiveQualitySupported = computed(() =>
    Boolean(mediaCenter.playback.value?.adaptive_quality_supported),
  );
  const playbackQualityOptions = computed(() =>
    buildPlaybackQualityOptions(
      sourceVideoHeightBaseline.value,
      QUALITY_DOWNGRADE_LEVELS,
      adaptiveQualitySupported.value,
      selectedQuality.value,
    ),
  );
  const overlayControls = usePlayerOverlayControls({
    hasSource,
    isBusy: mediaCenter.isBusy,
  });

  function readCachedSourceHeightBaseline(path: string): number | null {
    if (!path) {
      return null;
    }
    const value = sourceHeightBaselineByPath.value[path];
    return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
  }

  function writeCachedSourceHeightBaseline(path: string, height: number) {
    if (!path || !Number.isFinite(height) || height <= 0) {
      return;
    }
    const nextHeight = Math.round(height);
    const prevHeight = sourceHeightBaselineByPath.value[path];
    sourceHeightBaselineByPath.value[path] =
      typeof prevHeight === "number" && Number.isFinite(prevHeight) && prevHeight > 0
        ? Math.max(prevHeight, nextHeight)
        : nextHeight;
  }

  async function handlePlay() {
    await mediaCenter.play();
    playerErrorMessage.value = "";
  }

  async function handlePause(positionSeconds?: number) {
    if (typeof positionSeconds === "number" && Number.isFinite(positionSeconds)) {
      await mediaCenter.seek(positionSeconds);
    }
    await mediaCenter.pause();
  }

  async function handleStop() {
    if (mediaCenter.playback.value?.status === "playing") {
      await handlePause(mediaCenter.playback.value.position_seconds);
      return;
    }
    await mediaCenter.stop();
  }

  async function changePlaybackRate(rate: number) {
    playbackRate.value = rate;
    await mediaCenter.setRate(rate);
  }

  async function changeVolume(nextVolume: number) {
    const normalized = clamp(nextVolume, 0, 1);
    volume.value = normalized;
    muted.value = normalized <= 0;
    await mediaCenter.setVolume(normalized);
  }

  async function toggleMute() {
    muted.value = !muted.value;
    await mediaCenter.setMuted(muted.value);
  }

  async function changeQuality(nextQuality: string) {
    const nextMode = nextQuality as PlaybackQualityMode;
    selectedQuality.value = nextQuality;
    await mediaCenter.setQuality(nextMode);
  }

  function increasePlaybackRate() {
    const nextRate = Math.min(3, Number((playbackRate.value + 0.1).toFixed(1)));
    void changePlaybackRate(nextRate);
  }

  function decreasePlaybackRate() {
    const nextRate = Math.max(0.1, Number((playbackRate.value - 0.1).toFixed(1)));
    void changePlaybackRate(nextRate);
  }

  async function handlePlayFromUrlPlaylist(url: string) {
    mediaCenter.urlInputValue.value = url;
    await mediaCenter.openUrl(url);
    mediaCenter.urlDialogVisible.value = false;
  }

  usePlaybackShortcuts({
    playback: mediaCenter.playback,
    onPlay: () => void handlePlay(),
    onPause: (positionSeconds) => void handlePause(positionSeconds),
    onSeek: (positionSeconds) => void mediaCenter.seek(positionSeconds),
    onResetRate: () => void changePlaybackRate(1),
    onIncreaseRate: increasePlaybackRate,
    onDecreaseRate: decreasePlaybackRate,
  });

  watch(mediaCenter.currentSource, () => {
    playerErrorMessage.value = "";
    selectedQuality.value = "source";
    const path = mediaCenter.currentSource.value;
    sourceVideoHeightBaseline.value = path ? readCachedSourceHeightBaseline(path) : null;
  });

  watch(mediaCenter.playback, (value) => {
    if (!value) {
      selectedQuality.value = "source";
      return;
    }
    selectedQuality.value = value.quality_mode ?? "source";
    playbackRate.value = value.playback_rate ?? 1;
  });

  watch(mediaCenter.metadataVideoHeight, (nextHeight) => {
    if (typeof nextHeight !== "number" || !Number.isFinite(nextHeight) || nextHeight <= 0) {
      return;
    }
    if (sourceVideoHeightBaseline.value === null) {
      sourceVideoHeightBaseline.value = nextHeight;
    } else {
      sourceVideoHeightBaseline.value = Math.max(sourceVideoHeightBaseline.value, nextHeight);
    }
    if (mediaCenter.currentSource.value && sourceVideoHeightBaseline.value !== null) {
      writeCachedSourceHeightBaseline(
        mediaCenter.currentSource.value,
        sourceVideoHeightBaseline.value,
      );
    }
  });

  return {
    ...mediaCenter,
    ...overlayControls,
    playbackRate,
    volume,
    muted,
    selectedQuality,
    hasSource,
    displayErrorMessage,
    playbackQualityOptions,
    changePlaybackRate,
    changeVolume,
    changeQuality,
    handlePause,
    handlePlay,
    handlePlayFromUrlPlaylist,
    handleStop,
    toggleMute,
  };
}
