import { computed, ref, watch } from "vue";
import { useMediaCenter } from "../useMediaCenter";
import { usePlaybackQualityController } from "./usePlaybackQualityController";
import { usePlaybackTransportController } from "./usePlaybackTransportController";
import { usePlayerOverlayControls } from "./usePlayerOverlayControls";

export function useHomePlaybackController() {
  const mediaCenter = useMediaCenter();
  const playerErrorMessage = ref("");

  const displayErrorMessage = computed(
    () => playerErrorMessage.value || mediaCenter.errorMessage.value,
  );
  const hasSource = computed(() => Boolean(mediaCenter.currentSource.value));
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

  async function handlePlay() {
    await transportController.handlePlay();
    playerErrorMessage.value = "";
  }

  async function handlePlayFromUrlPlaylist(url: string) {
    mediaCenter.urlInputValue.value = url;
    await mediaCenter.openUrl(url);
    mediaCenter.urlDialogVisible.value = false;
  }

  watch(mediaCenter.currentSource, () => {
    playerErrorMessage.value = "";
  });

  return {
    ...mediaCenter,
    ...overlayControls,
    ...qualityController,
    ...transportController,
    handlePlay,
    hasSource,
    displayErrorMessage,
    handlePlayFromUrlPlaylist,
  };
}
