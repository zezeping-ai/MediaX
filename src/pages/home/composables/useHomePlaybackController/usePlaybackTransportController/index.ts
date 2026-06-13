import { ref, watch } from "vue";
import { normalizePlaybackRate } from "@/modules/player-constraints";
import type { PlaybackStatus } from "@/modules/media-types";
import { usePlaybackShortcuts } from "../usePlaybackShortcuts";
import { createTransportActions } from "./createTransportActions";
import type { UsePlaybackTransportControllerOptions } from "./types";

export function usePlaybackTransportController(options: UsePlaybackTransportControllerOptions) {
  const playbackRate = ref(1);
  const volume = ref(1);
  const muted = ref(false);
  const autoStoppedPath = ref("");
  const lastActivePath = ref("");
  const lastProgressAtTail = ref(false);
  const lastStatus = ref<PlaybackStatus>("idle");
  const LAST_FRAME_EPSILON_SECONDS = 0.12;

  const actions = createTransportActions({
    options,
    playbackRate,
    volume,
    muted,
  });

  function triggerTrackTailReached(path: string) {
    if (!path || autoStoppedPath.value === path) {
      return;
    }
    autoStoppedPath.value = path;
    if (options.onTrackTailReached) {
      void options.onTrackTailReached(path);
      return;
    }
    void options.stop();
  }

  usePlaybackShortcuts({
    playback: options.playback,
    onPlay: () => void actions.handlePlay(),
    onPause: (positionSeconds) => void actions.handlePause(positionSeconds),
    onSeek: (positionSeconds) => void options.seek(positionSeconds),
    onResetRate: () => void actions.changePlaybackRate(1),
    onIncreaseRate: actions.increasePlaybackRate,
    onDecreaseRate: actions.decreasePlaybackRate,
  });

  watch(options.playback, (value) => {
    if (!value) {
      playbackRate.value = 1;
      volume.value = 1;
      muted.value = false;
      autoStoppedPath.value = "";
      lastActivePath.value = "";
      lastProgressAtTail.value = false;
      lastStatus.value = "idle";
      return;
    }
    playbackRate.value = normalizePlaybackRate(value.playback_rate ?? 1);
    volume.value = value.volume ?? 1;
    muted.value = value.muted ?? false;

    const previousStatus = lastStatus.value;
    const status = value.status ?? "idle";
    lastStatus.value = status;

    const currentPath = value.current_path ?? "";
    const duration = Number.isFinite(value.duration_seconds) ? value.duration_seconds : 0;
    const position = Number.isFinite(value.position_seconds) ? value.position_seconds : 0;
    const isAtTail = duration > 0 && position >= Math.max(0, duration - LAST_FRAME_EPSILON_SECONDS);

    if (currentPath) {
      lastActivePath.value = currentPath;
      lastProgressAtTail.value = isAtTail;
    }

    if (!currentPath) {
      if (
        status === "stopped"
        && previousStatus === "playing"
        && lastActivePath.value
        && lastProgressAtTail.value
      ) {
        triggerTrackTailReached(lastActivePath.value);
        lastActivePath.value = "";
        lastProgressAtTail.value = false;
      }
      return;
    }

    const canTriggerAutoStop = status !== "stopped" && status !== "idle";
    if (isAtTail && canTriggerAutoStop && autoStoppedPath.value !== currentPath) {
      triggerTrackTailReached(currentPath);
      return;
    }
    if (!isAtTail && autoStoppedPath.value === currentPath) {
      autoStoppedPath.value = "";
    }
  });

  return {
    ...actions,
    muted,
    playbackRate,
    volume,
  };
}
