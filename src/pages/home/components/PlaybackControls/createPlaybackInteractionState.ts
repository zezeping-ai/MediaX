import { ref, watch } from "vue";
import { throttle } from "lodash-es";
import type { PlaybackControlsEmit, PlaybackControlsProps } from "./usePlaybackControlsViewModel";

export function createPlaybackInteractionState(
  props: PlaybackControlsProps,
  emit: PlaybackControlsEmit,
) {
  const speedDropdownOpen = ref(false);
  const qualityDropdownOpen = ref(false);
  const volumePreview = ref(props.volume);
  const adjustingVolume = ref(false);
  const emitThrottledVolumeChange = throttle((nextVolume: number) => {
    emit("change-volume", nextVolume);
  }, 48, {
    leading: true,
    trailing: true,
  });
  watch(
    () => speedDropdownOpen.value || qualityDropdownOpen.value,
    (open) => {
      emit("overlay-interaction-change", open);
    },
  );

  watch(
    () => props.volume,
    (nextVolume) => {
      if (!adjustingVolume.value) {
        volumePreview.value = nextVolume;
      }
    },
    { immediate: true },
  );

  function setSpeedDropdownOpen(value: boolean) {
    speedDropdownOpen.value = value;
    if (value) {
      qualityDropdownOpen.value = false;
    }
  }

  function setQualityDropdownOpen(value: boolean) {
    qualityDropdownOpen.value = value;
    if (value) {
      speedDropdownOpen.value = false;
    }
  }

  function emitVolumePreview(nextVolume: number) {
    adjustingVolume.value = true;
    volumePreview.value = nextVolume;
    emitThrottledVolumeChange(nextVolume);
  }

  function emitVolumeCommit(nextVolume: number) {
    emitThrottledVolumeChange.cancel();
    adjustingVolume.value = false;
    volumePreview.value = nextVolume;
    emit("change-volume", nextVolume);
  }

  function dispose() {
    emitThrottledVolumeChange.cancel();
    emit("overlay-interaction-change", false);
  }

  return {
    dispose,
    emitVolumeCommit,
    emitVolumePreview,
    qualityDropdownOpen,
    setQualityDropdownOpen,
    setSpeedDropdownOpen,
    speedDropdownOpen,
    volumePreview,
  };
}
