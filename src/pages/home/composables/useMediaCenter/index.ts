import { computed, onBeforeUnmount, ref, watch } from "vue";
import { usePreferences } from "@/modules/preferences";
import type { HardwareDecodeMode, PlaybackQualityMode } from "@/modules/media-types";
import { useCacheRecordingController } from "./useCacheRecordingController";
import { createMediaInfoSnapshot } from "./createMediaInfoSnapshot";
import { createPlaybackCommandRunner } from "./createPlaybackCommandRunner";
import { useMediaCenterBusyState } from "./useMediaCenterBusyState";
import { useMediaCenterLifecycle } from "./useMediaCenterLifecycle";
import { usePlayerPreferenceSync } from "./usePlayerPreferenceSync";
import { useMediaCommands } from "../useMediaCommands";
import { useMediaErrorMap } from "../useMediaErrorMap";
import { useMediaSession } from "../useMediaSession";
import { usePlaybackSettings } from "../usePlaybackSettings";
import { useMediaUrlInputController } from "./useMediaUrlInputController";

export function useMediaCenter() {
  const { playerHwDecodeMode, playerAlwaysOnTop, playerVideoScaleMode } = usePreferences();
  const mediaSession = useMediaSession();
  const commands = useMediaCommands();
  const { toUserErrorMessage } = useMediaErrorMap();
  const playbackSettings = usePlaybackSettings({
    configureDecoderMode: commands.configureDecoderMode,
    requestPreviewFrame: commands.requestPreviewFrame,
  });

  const { errorMessage, isBusy, withBusyState } = useMediaCenterBusyState(toUserErrorMessage);
  const recordingNoticeMessage = ref("");
  const lastSyncedSecond = ref(-1);
  const pendingSource = ref("");
  const playback = computed(() => mediaSession.snapshot.value?.playback ?? null);

  const cacheRecordingController = useCacheRecordingController({
    commands,
    currentSource: mediaSession.currentSource,
    metadataDurationSeconds: mediaSession.metadataDurationSeconds,
    playback,
    onErrorMessage: (message) => {
      errorMessage.value = message;
    },
    onNoticeMessage: (message) => {
      recordingNoticeMessage.value = message;
    },
  });

  const mediaInfoSnapshot = createMediaInfoSnapshot({
    playback,
    currentSource: mediaSession.currentSource,
    metadataDurationSeconds: mediaSession.metadataDurationSeconds,
    metadataVideoWidth: mediaSession.metadataVideoWidth,
    metadataVideoHeight: mediaSession.metadataVideoHeight,
    metadataVideoFps: mediaSession.metadataVideoFps,
  });

  const playbackRunner = createPlaybackCommandRunner({
    commands,
    playback,
    pendingSource,
    errorMessage,
    recordingNoticeMessage,
    lastSyncedSecond,
    toUserErrorMessage,
    updateSnapshot: mediaSession.updateSnapshot,
    refreshCacheRecordingStatus: cacheRecordingController.refreshCacheRecordingStatus,
  });

  async function applyHwDecodePreference(mode: HardwareDecodeMode) {
    if (playback.value?.hw_decode_mode === mode) {
      return;
    }
    const next = await playbackSettings.applyHwDecode(mode);
    if (next) {
      mediaSession.updateSnapshot(next);
    }
  }

  async function applyAlwaysOnTopPreference(enabled: boolean) {
    await playbackSettings.applyAlwaysOnTop(enabled);
  }

  async function applyVideoScaleModePreference(mode: "contain" | "cover") {
    await playbackSettings.applyVideoScaleMode(mode);
  }

  async function requestPreviewFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
    try {
      return await playbackSettings.requestTimelinePreview(positionSeconds, maxWidth, maxHeight);
    } catch {
      return null;
    }
  }

  const urlInputController = useMediaUrlInputController({
    openUrl: playbackRunner.openPath,
  });

  const mediaCenterLifecycle = useMediaCenterLifecycle({
    withBusyState,
    mediaSession,
    cacheRecordingController,
    playbackRunner,
    urlInputController,
  });

  usePlayerPreferenceSync({
    playerHwDecodeMode,
    playerAlwaysOnTop,
    playerVideoScaleMode,
    applyHwDecodePreference,
    applyAlwaysOnTopPreference,
    applyVideoScaleModePreference,
    onReady: async () => {
      await mediaCenterLifecycle.mountMediaCenter();
    },
  });

  onBeforeUnmount(() => {
    mediaSession.unmount();
  });

  watch(mediaSession.metadataDurationSeconds, (duration) => {
    if (typeof duration === "number" && Number.isFinite(duration) && duration > 0) {
      void playbackRunner.syncPosition(0, duration);
    }
  });

  watch(mediaSession.playbackErrorMessage, (message) => {
    if (message) {
      errorMessage.value = message;
    }
  });

  return {
    playback,
    currentSource: mediaSession.currentSource,
    pendingSource,
    effectiveDurationSeconds: cacheRecordingController.effectiveDurationSeconds,
    urlInputValue: urlInputController.urlInputValue,
    urlDialogVisible: urlInputController.urlDialogVisible,
    urlPlaylist: urlInputController.urlPlaylist,
    isBusy,
    cacheRecording: cacheRecordingController.cacheRecording,
    cacheOutputPath: cacheRecordingController.cacheOutputPath,
    cacheFinalizedOutputPath: cacheRecordingController.cacheFinalizedOutputPath,
    cacheOutputSizeBytes: cacheRecordingController.cacheOutputSizeBytes,
    cacheWriteSpeedBytesPerSecond: cacheRecordingController.cacheWriteSpeedBytesPerSecond,
    networkReadBytesPerSecond: mediaSession.networkReadBytesPerSecond,
    networkSustainRatio: mediaSession.networkSustainRatio,
    cacheOutputDir: cacheRecordingController.cacheOutputDir,
    errorMessage,
    recordingNoticeMessage,
    debugSnapshot: mediaSession.debugSnapshot,
    debugTimeline: mediaSession.debugTimeline,
    debugStageSnapshot: mediaSession.debugStageSnapshot,
    firstFrameAtMs: mediaSession.firstFrameAtMs,
    latestTelemetry: mediaSession.latestTelemetry,
    latestAudioMeter: mediaSession.latestAudioMeter,
    telemetryHistory: mediaSession.telemetryHistory,
    mediaInfoSnapshot,
    metadataMediaKind: mediaSession.metadataMediaKind,
    metadataTitle: mediaSession.metadataTitle,
    metadataArtist: mediaSession.metadataArtist,
    metadataAlbum: mediaSession.metadataAlbum,
    metadataHasCoverArt: mediaSession.metadataHasCoverArt,
    metadataLyrics: mediaSession.metadataLyrics,
    metadataVideoHeight: mediaSession.metadataVideoHeight,
    openLocalFileByDialog: () => withBusyState(async () => {
      const selectedPath = await playbackRunner.openLocalFileByDialog();
      if (selectedPath) {
        await playbackRunner.openPath(selectedPath);
      }
    }),
    openUrl: (url: string) => withBusyState(async () => {
      await urlInputController.submitUrl(url);
    }),
    requestOpenUrlInput: urlInputController.requestOpenUrlInput,
    cancelOpenUrlInput: urlInputController.cancelOpenUrlInput,
    confirmOpenUrlInput: () => withBusyState(urlInputController.confirmOpenUrlInput),
    removeUrlFromPlaylist: urlInputController.removeUrlFromPlaylist,
    clearUrlPlaylist: urlInputController.clearUrlPlaylist,
    play: () => withBusyState(playbackRunner.play),
    pause: () => withBusyState(playbackRunner.pause),
    stop: () => withBusyState(playbackRunner.stop),
    seek: (seconds: number) => playbackRunner.runWithoutBusyLock(() => playbackRunner.seek(seconds)),
    seekPreview: (seconds: number) => playbackRunner.seekPreview(seconds),
    setRate: (rate: number) => playbackRunner.runWithoutBusyLock(() => playbackRunner.setRate(rate)),
    setVolume: (volume: number) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setVolume(volume)),
    setMuted: (muted: boolean) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setMuted(muted)),
    setLeftChannelVolume: (volume: number) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setLeftChannelVolume(volume)),
    setRightChannelVolume: (volume: number) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setRightChannelVolume(volume)),
    setLeftChannelMuted: (muted: boolean) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setLeftChannelMuted(muted)),
    setRightChannelMuted: (muted: boolean) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setRightChannelMuted(muted)),
    setQuality: (mode: PlaybackQualityMode) => withBusyState(() => playbackRunner.setQuality(mode)),
    toggleCacheRecording: () => withBusyState(cacheRecordingController.toggleCacheRecording),
    requestPreviewFrame: (positionSeconds: number, maxWidth?: number, maxHeight?: number) =>
      requestPreviewFrame(positionSeconds, maxWidth, maxHeight),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      withBusyState(() => playbackRunner.syncPosition(positionSeconds, durationSeconds)),
  };
}
