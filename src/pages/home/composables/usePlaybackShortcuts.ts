import { useEventListener } from "@vueuse/core";
import type { Ref } from "vue";
import type { PlaybackState } from "@/modules/media-types";

type UsePlaybackShortcutsOptions = {
  playback: Ref<PlaybackState | null>;
  onPlay: () => void;
  onPause: (positionSeconds?: number) => void;
  onResetRate: () => void;
  onIncreaseRate: () => void;
  onDecreaseRate: () => void;
};

export function usePlaybackShortcuts(options: UsePlaybackShortcutsOptions) {
  useEventListener(window, "keydown", (event: KeyboardEvent) => {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
      return;
    }
    if (event.code === "Space") {
      event.preventDefault();
      if (options.playback.value?.status === "playing") {
        options.onPause(options.playback.value.position_seconds);
      } else {
        options.onPlay();
      }
      return;
    }
    if (event.key === "]") {
      event.preventDefault();
      options.onIncreaseRate();
      return;
    }
    if (event.key === "[") {
      event.preventDefault();
      options.onDecreaseRate();
      return;
    }
    if (event.key === "0") {
      event.preventDefault();
      options.onResetRate();
    }
  });
}
