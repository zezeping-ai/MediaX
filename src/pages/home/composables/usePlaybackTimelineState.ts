import { computed, onBeforeUnmount, ref, watch } from "vue";
import { throttle } from "lodash-es";
import type { PlaybackState } from "@/modules/media-types";
import { PREVIEW_SEEK_INTERVAL_MS } from "../components/PlaybackControls/playbackControls.constants";

interface UsePlaybackTimelineStateArgs {
  playback: () => PlaybackState | null;
  onSeek: (seconds: number) => void;
  onSeekPreview: (seconds: number) => void;
}

export function usePlaybackTimelineState({
  playback,
  onSeek,
  onSeekPreview,
}: UsePlaybackTimelineStateArgs) {
  const nowTick = ref(Date.now());
  const anchorPosition = ref(0);
  const anchorAtMs = ref(Date.now());
  let tickTimer: number | null = null;

  const currentTime = computed(() => {
    const value = playback();
    if (!value) {
      return 0;
    }
    if (value.status !== "playing") {
      return anchorPosition.value;
    }
    const rate = value.playback_rate > 0 ? value.playback_rate : 1;
    const elapsedSeconds = Math.max(0, nowTick.value - anchorAtMs.value) / 1000;
    const progressed = anchorPosition.value + elapsedSeconds * rate;
    const maxDuration = value.duration_seconds > 0 ? value.duration_seconds : Number.POSITIVE_INFINITY;
    return Math.min(progressed, maxDuration);
  });

  const emitPausedSeekPreview = throttle((seconds: number) => onSeekPreview(seconds), PREVIEW_SEEK_INTERVAL_MS, {
    leading: true,
    trailing: true,
  });

  function clearTickTimer() {
    if (tickTimer !== null) {
      window.clearInterval(tickTimer);
      tickTimer = null;
    }
  }

  function ensureTickTimer() {
    if (tickTimer !== null) {
      return;
    }
    tickTimer = window.setInterval(() => {
      nowTick.value = Date.now();
    }, 200);
  }

  function previewSeekWhilePaused(nextSeconds: number) {
    const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
    anchorPosition.value = normalized;
    anchorAtMs.value = Date.now();
    nowTick.value = anchorAtMs.value;
    if (playback()?.status !== "paused") {
      emitPausedSeekPreview.cancel();
      return;
    }
    emitPausedSeekPreview(normalized);
  }

  function commitSeek(nextSeconds: number) {
    const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
    // Slider release seek should win over any trailing paused preview seek.
    emitPausedSeekPreview.cancel();
    anchorPosition.value = normalized;
    anchorAtMs.value = Date.now();
    nowTick.value = anchorAtMs.value;
    onSeek(normalized);
  }

  watch(
    () => {
      const value = playback();
      return {
        status: value?.status,
        positionSeconds: value?.position_seconds ?? 0,
        playbackRate: value?.playback_rate ?? 1,
        durationSeconds: value?.duration_seconds ?? 0,
      };
    },
    (nextState) => {
      const value = playback();
      if (!value) {
        anchorPosition.value = 0;
        anchorAtMs.value = Date.now();
        clearTickTimer();
        return;
      }

      const backendPos = nextState.positionSeconds;
      const drift = backendPos - anchorPosition.value;
      const backendJumped = Math.abs(drift) >= 0.5;
      const backendAdvanced = drift > 0.05;
      if (backendJumped || backendAdvanced || value.status !== "playing") {
        anchorPosition.value = backendPos;
        anchorAtMs.value = Date.now();
      }

      if (value.status === "playing") {
        ensureTickTimer();
        emitPausedSeekPreview.cancel();
      } else {
        clearTickTimer();
      }
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    clearTickTimer();
    emitPausedSeekPreview.cancel();
  });

  return {
    currentTime,
    commitSeek,
    previewSeekWhilePaused,
    cancelPreviewSeek: () => emitPausedSeekPreview.cancel(),
  };
}
