import { computed, ref } from "vue";
import { applyLyricsFetchSettingsPreference, applyResumeLastPositionPreference } from "@/modules/player-settings-actions";
import { usePreferences } from "@/modules/preferences";
import type { HardwareDecodeMode, PreviewFrame } from "@/modules/media-types";
import type { VideoPictureTune } from "@/modules/video-picture-tune";
import { useCacheRecordingController } from "./useCacheRecordingController";
import { createMediaCenterActions } from "./createMediaCenterActions";
import { createMediaInfoSnapshot } from "./createMediaInfoSnapshot";
import { createPlaybackCommandRunner } from "./createPlaybackCommandRunner";
import { syncMediaCenterRuntime } from "./syncMediaCenterRuntime";
import { useMediaCenterBusyState } from "./useMediaCenterBusyState";
import { useMediaCenterLifecycle } from "./useMediaCenterLifecycle";
import { usePlayerPreferenceSync } from "./usePlayerPreferenceSync";
import { useMediaCommands } from "../useMediaCommands";
import { useMediaErrorMap } from "../useMediaErrorMap";
import { useMediaSession } from "../useMediaSession";
import { usePlaybackSettings } from "../usePlaybackSettings";
import { useMediaUrlInputController } from "./useMediaUrlInputController";
import { usePlaybackPlaylist } from "@/pages/home/composables/usePlaybackPlaylist";

export function useMediaCenter() {
  const {
    playerHwDecodeMode,
    playerAlwaysOnTop,
    playerVideoScaleMode,
    playerVideoPictureTune,
    playerResumeLastPosition,
    playerAutoFetchOnlineLyrics,
    playerLyricsProviders,
    playerLrcApiBaseUrl,
  } = usePreferences();
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
  const resumePromptPositionSeconds = ref<number | null>(null);
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
    resumeLastPosition: playerResumeLastPosition,
    resumePromptPositionSeconds,
  });

  const playlistController = usePlaybackPlaylist({
    currentSource: mediaSession.currentSource,
    metadataTitle: mediaSession.metadataTitle,
    openSource: (source) => playbackRunner.openPath(source),
    stopPlayback: () => playbackRunner.stop(),
  });

  async function openPathWithPlaylist(source: string) {
    await playbackRunner.openPath(source);
    playlistController.recordSourceOpened(source, mediaSession.metadataTitle.value);
  }

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

  async function applyVideoPictureTunePreference(tune: VideoPictureTune) {
    await playbackSettings.applyVideoPictureTune(tune);
  }

  async function requestPreviewFrame(
    positionSeconds: number,
    maxWidth = 160,
    maxHeight = 90,
  ): Promise<PreviewFrame | null> {
    try {
      return await playbackSettings.requestTimelinePreview(positionSeconds, maxWidth, maxHeight);
    } catch {
      return null;
    }
  }

  const urlInputController = useMediaUrlInputController({
    openUrl: openPathWithPlaylist,
    urlPlaylist: playlistController.urlPlaylist,
    removeUrlFromHistory: playlistController.removeUrlFromHistory,
    clearUrlHistory: playlistController.clearUrlHistory,
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
    playerVideoPictureTune,
    playerResumeLastPosition,
    playerAutoFetchOnlineLyrics,
    playerLyricsProviders,
    playerLrcApiBaseUrl,
    applyHwDecodePreference,
    applyAlwaysOnTopPreference,
    applyVideoScaleModePreference,
    applyVideoPictureTunePreference,
    applyResumeLastPositionPreference,
    applyLyricsFetchSettingsPreference,
    onReady: async () => {
      await mediaCenterLifecycle.mountMediaCenter();
    },
  });

  syncMediaCenterRuntime({
    mediaSession,
    playbackRunner,
    errorMessage,
  });

  const actions = createMediaCenterActions({
    withBusyState,
    playbackRunner,
    cacheRecordingController,
    urlInputController,
    playlistController,
    openPathWithPlaylist,
    currentSource: mediaSession.currentSource,
    requestPreviewFrame,
    resumePromptPositionSeconds,
    onNoticeMessage: (message) => {
      recordingNoticeMessage.value = message;
    },
  });

  async function acceptResumePrompt() {
    const positionSeconds = resumePromptPositionSeconds.value;
    if (positionSeconds == null) {
      return;
    }
    resumePromptPositionSeconds.value = null;
    await actions.seek(positionSeconds);
  }

  function dismissResumePrompt() {
    resumePromptPositionSeconds.value = null;
  }

  return {
    playback,
    initialized: mediaSession.initialized,
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
    resumePromptPositionSeconds,
    acceptResumePrompt,
    dismissResumePrompt,
    latestAudioMeter: mediaSession.latestAudioMeter,
    mediaInfoSnapshot,
    metadataMediaKind: mediaSession.metadataMediaKind,
    metadataTitle: mediaSession.metadataTitle,
    metadataArtist: mediaSession.metadataArtist,
    metadataAlbum: mediaSession.metadataAlbum,
    metadataHasCoverArt: mediaSession.metadataHasCoverArt,
    metadataLyrics: mediaSession.metadataLyrics,
    metadataLyricsSource: mediaSession.metadataLyricsSource,
    metadataLyricsCandidateId: mediaSession.metadataLyricsCandidateId,
    metadataLyricsCandidates: mediaSession.metadataLyricsCandidates,
    metadataLyricsFetching: mediaSession.metadataLyricsFetching,
    metadataVideoHeight: mediaSession.metadataVideoHeight,
    updatePlaybackSnapshot: mediaSession.updateSnapshot,
    playlistController,
    ...actions,
  };
}
