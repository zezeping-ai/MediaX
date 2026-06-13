import { computed } from "vue";
import type { useHomePageViewModel } from "./useHomePageViewModel";
import type {
  MediaViewportEventMap,
  PlaybackControlsEventMap,
  UrlDialogEventMap,
} from "./homePageBindings.types";

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
    initialized: viewModel.initialized.value,
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

  const mediaViewportEvents: MediaViewportEventMap = {
    ended: viewModel.handleVideoEnded,
    "quick-open-local": viewModel.openLocalFileByDialog,
    "quick-open-url": viewModel.requestOpenUrlInput,
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
    playlistOpen: viewModel.playlistController.panelVisible.value,
    queueCount: viewModel.playlistController.queueCount.value,
    resumePromptPositionSeconds: viewModel.resumePromptPositionSeconds.value,
    requestPreviewFrame: viewModel.requestPreviewFrame,
  }));

  const playbackControlsEvents: PlaybackControlsEventMap = {
    mouseenter: viewModel.onControlsMouseEnter,
    mouseleave: viewModel.onControlsMouseLeave,
    mousemove: viewModel.markMouseActive,
    play: viewModel.handlePlay,
    pause: (position: number) => viewModel.handlePause(position),
    stop: viewModel.handleStop,
    seek: viewModel.seek,
    "seek-preview": viewModel.seekPreview,
    "change-rate": (value: number) => void viewModel.changePlaybackRate(value),
    "change-volume": (value: number) => void viewModel.changeVolume(value),
    "change-quality": (value: string) => void viewModel.changeQuality(value),
    "overlay-interaction-change": viewModel.setControlsOverlayInteracting,
    "toggle-mute": () => void viewModel.toggleMute(),
    "set-left-channel-volume": (value: number) => void viewModel.setLeftChannelVolume(value),
    "set-right-channel-volume": (value: number) => void viewModel.setRightChannelVolume(value),
    "set-left-channel-muted": (value: boolean) => void viewModel.setLeftChannelMuted(value),
    "set-right-channel-muted": (value: boolean) => void viewModel.setRightChannelMuted(value),
    "set-channel-routing": (value: string) => void viewModel.setChannelRouting(value as never),
    "toggle-cache": viewModel.toggleCacheRecording,
    "toggle-lock": viewModel.toggleLock,
    "toggle-playlist": () => viewModel.togglePlaylistPanel(),
    "export-audio": viewModel.exportCurrentAudio,
    "resume-prompt-accept": () => void viewModel.acceptResumePrompt(),
    "resume-prompt-dismiss": viewModel.dismissResumePrompt,
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

  const urlDialogEvents: UrlDialogEventMap = {
    confirm: viewModel.confirmOpenUrlInput,
    cancel: viewModel.cancelOpenUrlInput,
    clear: viewModel.clearUrlPlaylist,
    remove: viewModel.removeUrlFromPlaylist,
    select: (url: string) => {
      viewModel.urlInputValue.value = url;
    },
    play: viewModel.handlePlayFromUrlPlaylist,
  };

  const playlistPanelProps = computed(() => ({
    queue: viewModel.playlistController.queue.value,
    history: viewModel.playlistController.history.value,
    currentPlayingId: viewModel.playlistController.currentPlayingId.value,
    busy: viewModel.isBusy.value,
    hasNext: viewModel.playlistController.hasNext.value,
    hasPrevious: viewModel.playlistController.hasPrevious.value,
    queueCount: viewModel.playlistController.queueCount.value,
    advanceMode: viewModel.playlistController.advanceMode.value,
  }));

  const playlistPanelEvents = {
    playQueue: (id: string) => void viewModel.playQueueItem(id),
    playHistory: (id: string) => void viewModel.playHistoryItem(id),
    removeQueue: viewModel.removeQueueItem,
    removeHistory: viewModel.removeHistoryItem,
    addToQueue: viewModel.addToQueue,
    reorder: viewModel.reorderQueue,
    clearQueue: viewModel.clearQueue,
    clearHistory: viewModel.clearHistory,
    playNext: () => void viewModel.playNextInQueue(),
    playPrevious: () => void viewModel.playPreviousInQueue(),
    importFiles: () => void viewModel.importLocalFilesToQueue(),
    importFolder: () => void viewModel.importFolderToQueue(),
    changeAdvanceMode: viewModel.setAdvanceMode,
  };

  return {
    controlsVisible: viewModel.controlsVisible,
    hasSource: viewModel.hasSource,
    mediaViewportEvents,
    mediaViewportProps,
    playbackControlsEvents,
    playbackControlsProps,
    playlistPanelEvents,
    playlistPanelProps,
    playlistPanelVisible: viewModel.playlistController.panelVisible,
    shellEvents,
    statusAlertProps,
    urlDialogInputValue: viewModel.urlInputValue,
    urlDialogEvents,
    urlDialogProps,
    urlDialogVisible: viewModel.urlDialogVisible,
  };
}
