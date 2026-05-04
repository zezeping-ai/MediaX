import { computed } from "vue";
import { useMediaCenter } from "../useMediaCenter";
import { useHomePlaybackActions } from "./useHomePlaybackActions";
import { usePlaybackQualityController } from "./usePlaybackQualityController";
import { usePlaybackTransportController } from "./usePlaybackTransportController";
import { usePlayerOverlayControls } from "./usePlayerOverlayControls";

export function useHomePlaybackController() {
  const mediaCenter = useMediaCenter();
  const hasSource = computed(() => {
    const status = mediaCenter.playback.value?.status ?? "idle";
    if (status === "idle" || status === "stopped") {
      return false;
    }
    return Boolean(mediaCenter.currentSource.value || mediaCenter.pendingSource.value);
  });
  const overlayControls = usePlayerOverlayControls({
    hasSource,
    isBusy: mediaCenter.isBusy,
  });
  const qualityController = usePlaybackQualityController({
    currentSource: mediaCenter.currentSource,
    playback: mediaCenter.playback,
    metadataVideoHeight: mediaCenter.metadataVideoHeight,
    setQuality: mediaCenter.setQuality,
  });
  const transportController = usePlaybackTransportController({
    playback: mediaCenter.playback,
    play: mediaCenter.play,
    pause: mediaCenter.pause,
    stop: mediaCenter.stop,
    seek: mediaCenter.seek,
    setRate: mediaCenter.setRate,
    setVolume: mediaCenter.setVolume,
    setMuted: mediaCenter.setMuted,
  });
  const playbackActions = useHomePlaybackActions({
    currentSource: mediaCenter.currentSource,
    errorMessage: mediaCenter.errorMessage,
    urlInputValue: mediaCenter.urlInputValue,
    urlDialogVisible: mediaCenter.urlDialogVisible,
    openUrl: mediaCenter.openUrl,
    handleTransportPlay: transportController.handlePlay,
  });

  return {
    ...mediaCenter,
    ...overlayControls,
    ...qualityController,
    ...transportController,
    ...playbackActions,
    hasSource,
  };
}
