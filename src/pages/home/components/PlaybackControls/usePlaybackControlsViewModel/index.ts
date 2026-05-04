import { computed, onBeforeUnmount } from "vue";
import { usePlaybackTimelineState } from "../../../composables/usePlaybackTimelineState";
import { createPlaybackDerivedState } from "../createPlaybackDerivedState";
import { createPlaybackInteractionState } from "../createPlaybackInteractionState";
import { createPlaybackActionHandlers } from "./createPlaybackActionHandlers";
import { createPlaybackTimelineHandlers } from "./createPlaybackTimelineHandlers";
import type { PlaybackControlsEmit, PlaybackControlsProps } from "./types";

export type {
  PlaybackControlsEmit,
  PlaybackControlsProps,
  RequestPreviewFrame,
} from "./types";

export type PlaybackControlsViewModel = ReturnType<typeof usePlaybackControlsViewModel>;

export function usePlaybackControlsViewModel(
  props: PlaybackControlsProps,
  emit: PlaybackControlsEmit,
) {
  const { currentTime, commitSeek, previewSeekWhilePaused, cancelPreviewSeek } =
    usePlaybackTimelineState({
      playback: () => props.playback,
      progressPosition: () => props.progressPositionSecondsOverride,
      onSeek: (seconds) => emit("seek", seconds),
      onSeekPreview: (seconds) => emit("seek-preview", seconds),
    });

  const duration = computed(() => {
    const base = props.playback?.duration_seconds ?? 0;
    const override = props.durationSecondsOverride ?? 0;
    const normalizedBase = Number.isFinite(base) ? Math.max(0, base) : 0;
    const normalizedOverride = Number.isFinite(override) ? Math.max(0, override) : 0;
    return Math.max(normalizedBase, normalizedOverride);
  });
  const bufferedPosition = computed(() => {
    const base = props.playback?.buffered_position_seconds ?? 0;
    const override = props.bufferedPositionSecondsOverride ?? 0;
    const normalizedBase = Number.isFinite(base) ? Math.max(0, base) : 0;
    const normalizedOverride = Number.isFinite(override) ? Math.max(0, override) : 0;
    return Math.max(normalizedBase, normalizedOverride, currentTime.value);
  });
  const interactionState = createPlaybackInteractionState(props, emit);
  const derivedState = createPlaybackDerivedState(
    props,
    duration,
    currentTime,
    interactionState.volumePreview,
  );

  const timelineHandlers = createPlaybackTimelineHandlers({
    canSeek: derivedState.canSeek,
    commitSeek,
    previewSeekWhilePaused,
  });
  const actionHandlers = createPlaybackActionHandlers({
    emit,
    setSpeedDropdownOpen: interactionState.setSpeedDropdownOpen,
    setQualityDropdownOpen: interactionState.setQualityDropdownOpen,
    emitVolumePreview: interactionState.emitVolumePreview,
    emitVolumeCommit: interactionState.emitVolumeCommit,
  });

  onBeforeUnmount(() => {
    cancelPreviewSeek();
    interactionState.dispose();
  });

  return {
    cacheIcon: derivedState.cacheIcon,
    bufferedPosition,
    currentTime,
    decodeBadgeClass: derivedState.decodeBadgeClass,
    decodeBadgeLabel: derivedState.decodeBadgeLabel,
    decodeBadgeTitle: derivedState.decodeBadgeTitle,
    duration,
    ...timelineHandlers,
    ...actionHandlers,
    isPlaying: derivedState.isPlaying,
    lockIcon: derivedState.lockIcon,
    qualityDropdownOpen: interactionState.qualityDropdownOpen,
    qualityLabel: derivedState.qualityLabel,
    setQualityDropdownOpen: interactionState.setQualityDropdownOpen,
    setSpeedDropdownOpen: interactionState.setSpeedDropdownOpen,
    sliderMax: derivedState.sliderMax,
    speedDropdownOpen: interactionState.speedDropdownOpen,
    timelineDisabled: derivedState.timelineDisabled,
    timelineTitle: derivedState.timelineTitle,
    volumePreview: interactionState.volumePreview,
    volumeIcon: derivedState.volumeIcon,
  };
}
