import { ref, watch } from "vue";
import { usePlaybackShortcuts } from "../usePlaybackShortcuts";
import { createTransportActions } from "./createTransportActions";
import type { UsePlaybackTransportControllerOptions } from "./types";

export function usePlaybackTransportController(options: UsePlaybackTransportControllerOptions) {
  const playbackRate = ref(1);
  const volume = ref(1);
  const muted = ref(false);

  const actions = createTransportActions({
    options,
    playbackRate,
    volume,
    muted,
  });

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
      return;
    }
    playbackRate.value = value.playback_rate ?? 1;
    volume.value = value.volume ?? 1;
    muted.value = value.muted ?? false;
  });

  return {
    ...actions,
    muted,
    playbackRate,
    volume,
  };
}
