import type { ReturnTypeComputedRef } from "./internalTypes";
import { normalizeSliderValue } from "./sliderValue";
import type { SliderValue } from "./types";

interface TimelineHandlersOptions {
  canSeek: ReturnTypeComputedRef<boolean>;
  commitSeek: (seconds: number) => void;
  previewSeekWhilePaused: (seconds: number) => void;
}

export function createPlaybackTimelineHandlers(options: TimelineHandlersOptions) {
  function handleProgressPreviewUpdate(value: SliderValue) {
    if (!options.canSeek.value) {
      return;
    }
    options.previewSeekWhilePaused(normalizeSliderValue(value));
  }

  function handleProgressCommit(value: SliderValue) {
    if (!options.canSeek.value) {
      return;
    }
    options.commitSeek(normalizeSliderValue(value));
  }

  return {
    handleProgressCommit,
    handleProgressPreviewUpdate,
  };
}
