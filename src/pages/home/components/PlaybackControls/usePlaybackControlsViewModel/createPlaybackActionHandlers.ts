import { normalizeSliderValue } from "./sliderValue";
import type { SliderValue, PlaybackControlsEmit } from "./types";

interface PlaybackActionHandlersOptions {
  emit: PlaybackControlsEmit;
  setSpeedDropdownOpen: (value: boolean) => void;
  setQualityDropdownOpen: (value: boolean) => void;
  emitVolumePreview: (value: number) => void;
  emitVolumeCommit: (value: number) => void;
}

export function createPlaybackActionHandlers(options: PlaybackActionHandlersOptions) {
  function handleSpeedChange(key: string | number) {
    options.setSpeedDropdownOpen(false);
    options.emit("change-rate", Number(key));
  }

  function handleQualityChange(key: string | number) {
    options.setQualityDropdownOpen(false);
    options.emit("change-quality", String(key));
  }

  function handleVolumeChange(value: SliderValue) {
    options.emitVolumePreview(normalizeSliderValue(value));
  }

  function handleVolumeCommit(value: SliderValue) {
    options.emitVolumeCommit(normalizeSliderValue(value));
  }

  return {
    handleQualityChange,
    handleSpeedChange,
    handleVolumeChange,
    handleVolumeCommit,
  };
}
