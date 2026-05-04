import { computed, onBeforeUnmount, ref, watch } from "vue";
import { throttle } from "lodash-es";
import type { PlaybackState } from "@/modules/media-types";
import { PREVIEW_SEEK_INTERVAL_MS } from "../../components/PlaybackControls/playbackControls.constants";

interface UsePlaybackTimelineStateArgs {
  playback: () => PlaybackState | null;
  progressPosition: () => number | null | undefined;
  onSeek: (seconds: number) => void;
  onSeekPreview: (seconds: number) => void;
}

export function usePlaybackTimelineState({
  playback,
  progressPosition,
  onSeek,
  onSeekPreview,
}: UsePlaybackTimelineStateArgs) {
  const anchorPosition = ref(0);
  const scrubbing = ref(false);
  const pendingSeekTarget = ref<number | null>(null);
  const pendingSeekStartedAt = ref(0);
  const SEEK_COMMIT_SETTLE_TIMEOUT_MS = 1600;
  const SEEK_COMMIT_SETTLE_TOLERANCE_SECONDS = 0.45;

  const currentTime = computed(() => anchorPosition.value);

  const emitPausedSeekPreview = throttle((seconds: number) => onSeekPreview(seconds), PREVIEW_SEEK_INTERVAL_MS, {
    leading: true,
    trailing: true,
  });

  function previewSeekWhilePaused(nextSeconds: number) {
    const normalized = Math.max(0, Number.isFinite(nextSeconds) ? nextSeconds : 0);
    scrubbing.value = true;
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
    scrubbing.value = false;
    anchorPosition.value = normalized;
    pendingSeekTarget.value = normalized;
    pendingSeekStartedAt.value = Date.now();
    onSeek(normalized);
  }

  function cancelPreviewSeek() {
    scrubbing.value = false;
    pendingSeekTarget.value = null;
    emitPausedSeekPreview.cancel();
  }

  watch(
    () => {
      const value = playback();
      const override = progressPosition();
      return {
        status: value?.status,
        positionSeconds: value?.position_seconds ?? 0,
        progressPositionSeconds:
          typeof override === "number" && Number.isFinite(override) ? Math.max(0, override) : null,
        playbackRate: value?.playback_rate ?? 1,
        durationSeconds: value?.duration_seconds ?? 0,
      };
    },
    (nextState) => {
      const value = playback();
      if (!value) {
        anchorPosition.value = 0;
        scrubbing.value = false;
        return;
      }

      if (scrubbing.value) {
        return;
      }

      const backendPos = nextState.progressPositionSeconds ?? nextState.positionSeconds;
      const pendingTarget = pendingSeekTarget.value;
      if (pendingTarget != null) {
        const settled = Math.abs(backendPos - pendingTarget) <= SEEK_COMMIT_SETTLE_TOLERANCE_SECONDS;
        const timedOut = Date.now() - pendingSeekStartedAt.value >= SEEK_COMMIT_SETTLE_TIMEOUT_MS;
        if (!settled && !timedOut) {
          emitPausedSeekPreview.cancel();
          return;
        }
        pendingSeekTarget.value = null;
      }
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
    cancelPreviewSeek();
  });

  return {
    currentTime,
    commitSeek,
    previewSeekWhilePaused,
    cancelPreviewSeek,
  };
}
