import { computed } from "vue";
import type { useHomePageViewModel } from "./useHomePageViewModel";

type HomePageViewModel = ReturnType<typeof useHomePageViewModel>;

export function useHomePageBindings(viewModel: HomePageViewModel) {
  const shellEvents = {
    onPointerMove: viewModel.markMouseActive,
    onPointerActivate: viewModel.markMouseActive,
    onFocusIn: viewModel.markMouseActive,
    onPointerLeave: viewModel.hideControlsImmediately,
  };

  const mediaViewportProps = computed(() => ({
    source: viewModel.currentSource.value,
    pendingSource: viewModel.pendingSource.value,
    controlsVisible: viewModel.controlsVisible.value,
    playback: viewModel.playback.value,
    loading: viewModel.isBusy.value,
    latestAudioMeter: viewModel.latestAudioMeter.value,
    metadataAlbum: viewModel.metadataAlbum.value,
    metadataArtist: viewModel.metadataArtist.value,
    metadataHasCoverArt: viewModel.metadataHasCoverArt.value,
    metadataLyrics: viewModel.metadataLyrics.value,
    metadataMediaKind: viewModel.metadataMediaKind.value,
    metadataTitle: viewModel.metadataTitle.value,
    setLeftChannelMuted: viewModel.setLeftChannelMuted,
    setChannelRouting: viewModel.setChannelRouting,
    setLeftChannelVolume: viewModel.setLeftChannelVolume,
    setRightChannelMuted: viewModel.setRightChannelMuted,
    setRightChannelVolume: viewModel.setRightChannelVolume,
    networkReadBytesPerSecond: viewModel.networkReadBytesPerSecond.value,
    networkSustainRatio: viewModel.networkSustainRatio.value,
    cacheRecording: viewModel.cacheRecording.value,
    cacheOutputPath: viewModel.cacheOutputPath.value,
    cacheOutputSizeBytes: viewModel.cacheOutputSizeBytes.value,
    cacheWriteSpeedBytesPerSecond: viewModel.cacheWriteSpeedBytesPerSecond.value,
  }));

  const mediaViewportEvents = {
    onEnded: viewModel.handleVideoEnded,
    onQuickOpenLocal: viewModel.openLocalFileByDialog,
    onQuickOpenUrl: viewModel.requestOpenUrlInput,
  };

  const playbackControlsProps = computed(() => ({
    playback: viewModel.playback.value,
    playbackRate: viewModel.playbackRate.value,
    volume: viewModel.volume.value,
    muted: viewModel.muted.value,
    locked: viewModel.controlsLocked.value,
    cacheRecording: viewModel.cacheRecording.value,
    cacheOutputPath: viewModel.cacheOutputPath.value,
    showAudioExport: viewModel.playback.value?.media_kind === "video",
    durationSecondsOverride: viewModel.effectiveDurationSeconds.value,
    bufferedPositionSecondsOverride: viewModel.playback.value?.buffered_position_seconds ?? 0,
    qualityOptions: viewModel.playbackQualityOptions.value,
    selectedQuality: viewModel.selectedQuality.value,
    disabled: !viewModel.currentSource.value || viewModel.isBusy.value,
    requestPreviewFrame: viewModel.requestPreviewFrame,
  }));

  const playbackControlsEvents = {
    onMouseEnter: viewModel.onControlsMouseEnter,
    onMouseLeave: viewModel.onControlsMouseLeave,
    onMouseMove: viewModel.markMouseActive,
    onPlay: viewModel.handlePlay,
    onPause: (position: number) => viewModel.handlePause(position),
    onStop: viewModel.handleStop,
    onSeek: viewModel.seek,
    onSeekPreview: viewModel.seekPreview,
    onChangeRate: (value: number) => void viewModel.changePlaybackRate(value),
    onChangeVolume: (value: number) => void viewModel.changeVolume(value),
    onChangeQuality: (value: string) => void viewModel.changeQuality(value),
    onOverlayInteractionChange: viewModel.setControlsOverlayInteracting,
    onToggleMute: () => void viewModel.toggleMute(),
    onSetLeftChannelVolume: (value: number) => void viewModel.setLeftChannelVolume(value),
    onSetRightChannelVolume: (value: number) => void viewModel.setRightChannelVolume(value),
    onSetLeftChannelMuted: (value: boolean) => void viewModel.setLeftChannelMuted(value),
    onSetRightChannelMuted: (value: boolean) => void viewModel.setRightChannelMuted(value),
    onSetChannelRouting: (value: string) => void viewModel.setChannelRouting(value as never),
    onToggleCache: viewModel.toggleCacheRecording,
    onToggleLock: viewModel.toggleLock,
    onExportAudio: viewModel.exportCurrentAudio,
  };

  const statusAlertProps = computed(() => ({
    cacheFinalizedOutputPath: viewModel.cacheFinalizedOutputPath.value,
    recordingNoticeMessage: viewModel.recordingNoticeMessage.value,
    displayErrorMessage: viewModel.displayErrorMessage.value,
  }));

  const urlDialogProps = computed(() => ({
    busy: viewModel.isBusy.value,
    playlist: viewModel.urlPlaylist.value,
  }));

  const urlDialogEvents = {
    onConfirm: viewModel.confirmOpenUrlInput,
    onCancel: viewModel.cancelOpenUrlInput,
    onClear: viewModel.clearUrlPlaylist,
    onRemove: viewModel.removeUrlFromPlaylist,
    onSelect: (url: string) => {
      viewModel.urlInputValue.value = url;
    },
    onPlay: viewModel.handlePlayFromUrlPlaylist,
  };

  return {
    controlsVisible: viewModel.controlsVisible,
    hasSource: viewModel.hasSource,
    mediaViewportEvents,
    mediaViewportProps,
    playbackControlsEvents,
    playbackControlsProps,
    shellEvents,
    statusAlertProps,
    urlDialogInputValue: viewModel.urlInputValue,
    urlDialogEvents,
    urlDialogProps,
    urlDialogVisible: viewModel.urlDialogVisible,
  };
}
