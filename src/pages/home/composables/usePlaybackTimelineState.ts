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
  const anchorPosition = ref(0);

  const currentTime = computed(() => anchorPosition.value);

  const emitPausedSeekPreview = throttle((seconds: number) => onSeekPreview(seconds), PREVIEW_SEEK_INTERVAL_MS, {
    leading: true,
    trailing: true,
  });

  function previewSeekWhilePaused(nextSeconds: number) {
    const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
    anchorPosition.value = normalized;
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
        return;
      }

      const backendPos = nextState.positionSeconds;
      const drift = backendPos - anchorPosition.value;
      const backendJumped = Math.abs(drift) >= 0.5;
      const backendAdvanced = drift > 0.05;
      if (backendJumped || backendAdvanced || value.status !== "playing") {
        anchorPosition.value = backendPos;
      }
      emitPausedSeekPreview.cancel();
    },
    { immediate: true },
  );

  onBeforeUnmount(() => {
    emitPausedSeekPreview.cancel();
  });

  return {
    currentTime,
    commitSeek,
    previewSeekWhilePaused,
    cancelPreviewSeek: () => emitPausedSeekPreview.cancel(),
  };
}
