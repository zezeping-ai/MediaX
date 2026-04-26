import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { usePreferences } from "@/modules/preferences";
import { getPlaybackSnapshot } from "@/modules/media-player";
import type { HardwareDecodeMode, MediaSnapshot, PlaybackQualityMode } from "@/modules/media-types";
import { useCacheRecordingController } from "./useCacheRecordingController";
import { useMediaCommands } from "../useMediaCommands";
import { useMediaErrorMap } from "../useMediaErrorMap";
import { useMediaUrlInputController } from "./useMediaUrlInputController";
import { usePlaybackSettings } from "../usePlaybackSettings";
import { useMediaSession } from "../useMediaSession";
const DEV_SEEK_LOG = import.meta.env.DEV;

export function useMediaCenter() {
  const { playerHwDecodeMode, playerAlwaysOnTop, playerVideoScaleMode } = usePreferences();
  const {
    snapshot,
    currentSource,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    firstFrameAtMs,
    latestTelemetry,
    telemetryHistory,
    networkReadBytesPerSecond,
    networkSustainRatio,
    metadataDurationSeconds,
    metadataVideoWidth,
    metadataVideoHeight,
    metadataVideoFps,
    playbackErrorMessage,
    mount,
    unmount,
    updateSnapshot,
  } = useMediaSession();
  const commands = useMediaCommands();
  const { toUserErrorMessage } = useMediaErrorMap();
  const playbackSettings = usePlaybackSettings({
    configureDecoderMode: commands.configureDecoderMode,
    requestPreviewFrame: commands.requestPreviewFrame,
  });
  const isBusy = ref(false);
  const errorMessage = ref("");
  const recordingNoticeMessage = ref("");
  const lastSyncedSecond = ref(-1);
  const pendingSource = ref("");

  const playback = computed(() => snapshot.value?.playback ?? null);
  const cacheRecordingController = useCacheRecordingController({
    commands,
    currentSource,
    metadataDurationSeconds,
    playback,
    onErrorMessage: (message) => {
      errorMessage.value = message;
    },
    onNoticeMessage: (message) => {
      recordingNoticeMessage.value = message;
    },
  });

  const mediaInfoSnapshot = computed<Record<string, string>>(() => {
    const playbackState = playback.value;
    const source = currentSource.value;
    const duration =
      playbackState?.duration_seconds ||
      metadataDurationSeconds.value ||
      0;
    const width = metadataVideoWidth.value || 0;
    const height = metadataVideoHeight.value || 0;
    const fps = metadataVideoFps.value || 0;

    const record: Record<string, string> = {};
    if (source) record.source = source;
    if (playbackState?.engine) record.engine = playbackState.engine;
    if (duration > 0) record.duration = `${duration.toFixed(3)}s`;
    if (width > 0 && height > 0) record.resolution = `${width}x${height}`;
    if (fps > 0) record.fps = `${fps.toFixed(3)}fps`;
    if (playbackState?.quality_mode) record.quality = playbackState.quality_mode;
    return record;
  });

  async function runPlaybackCommand(command: () => Promise<MediaSnapshot>) {
    const next = await command();
    updateSnapshot(next);
    return next;
  }

  async function refreshSnapshot() {
    updateSnapshot(await getPlaybackSnapshot());
  }

  async function applyHwDecodePreference(mode: HardwareDecodeMode) {
    if (playback.value?.hw_decode_mode === mode) {
      return;
    }
    const next = await playbackSettings.applyHwDecode(mode);
    if (next) {
      updateSnapshot(next);
    }
  }

  async function applyAlwaysOnTopPreference(enabled: boolean) {
    await playbackSettings.applyAlwaysOnTop(enabled);
  }

  async function applyVideoScaleModePreference(mode: "contain" | "cover") {
    await playbackSettings.applyVideoScaleMode(mode);
  }

  async function openLocalFileByDialog() {
    const selected = await open({
      title: "选择本地视频文件",
      multiple: false,
      filters: [
        {
          name: "Video",
          extensions: ["mp4", "mkv", "mov", "avi", "webm", "m4v", "mpeg", "mpg", "ts"],
        },
      ],
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }
    await openPath(selected);
  }

  async function openPath(path: string) {
    pendingSource.value = path;
    try {
      await runPlaybackCommand(() => commands.openPath(path));
      await runPlaybackCommand(commands.play);
      await cacheRecordingController.refreshCacheRecordingStatus();
      recordingNoticeMessage.value = "";
      errorMessage.value = "";
    } finally {
      pendingSource.value = "";
    }
  }
  const urlInputController = useMediaUrlInputController({
    openUrl: openPath,
  });

  async function play() {
    await runPlaybackCommand(commands.play);
  }

  async function pause() {
    await runPlaybackCommand(commands.pause);
  }

  async function stop() {
    await runPlaybackCommand(commands.stop);
    await cacheRecordingController.refreshCacheRecordingStatus();
  }

  async function seek(positionSeconds: number) {
    const status = playback.value?.status ?? "unknown";
    const forceRender = status === "paused";
    logSeekDecision("seek", positionSeconds, forceRender, status);
    await runPlaybackCommand(() => commands.seek(positionSeconds, forceRender));
  }

  async function seekPreview(positionSeconds: number) {
    // Scrubbing preview should stay responsive and not lock controls with busy state.
    try {
      const status = playback.value?.status ?? "unknown";
      logSeekDecision("seekPreview", positionSeconds, false, status);
      await runPlaybackCommand(() => commands.seek(positionSeconds, false));
    } catch (error) {
      errorMessage.value = toUserErrorMessage(error);
    }
  }

  async function setRate(playbackRate: number) {
    await runPlaybackCommand(() => commands.setRate(playbackRate));
  }

  async function setVolume(volume: number) {
    await runPlaybackCommand(() => commands.setVolume(volume));
  }

  async function setMuted(muted: boolean) {
    await runPlaybackCommand(() => commands.setMuted(muted));
  }

  async function setQuality(mode: PlaybackQualityMode) {
    await runPlaybackCommand(() => commands.setQuality(mode));
  }

  async function requestPreviewFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
    try {
      return await playbackSettings.requestTimelinePreview(positionSeconds, maxWidth, maxHeight);
    } catch {
      return null;
    }
  }

  async function syncPosition(positionSeconds: number, durationSeconds: number) {
    const second = Math.floor(positionSeconds);
    if (second === lastSyncedSecond.value) {
      return;
    }
    lastSyncedSecond.value = second;
    await runPlaybackCommand(() => commands.syncPosition(positionSeconds, durationSeconds));
  }

  async function withBusyState(action: () => Promise<void>) {
    isBusy.value = true;
    try {
      await action();
    } catch (error) {
      errorMessage.value = toUserErrorMessage(error);
    } finally {
      isBusy.value = false;
    }
  }

  onMounted(async () => {
    await withBusyState(refreshSnapshot);
    // Ensure backend matches persisted preference.
    await applyHwDecodePreference(playerHwDecodeMode.value);
    await applyAlwaysOnTopPreference(playerAlwaysOnTop.value);
    await applyVideoScaleModePreference(playerVideoScaleMode.value);
    await cacheRecordingController.refreshCacheRecordingStatus();
    if (cacheRecordingController.cacheRecording.value) {
      cacheRecordingController.startRecordingClock();
      cacheRecordingController.startCacheStatusPoll();
    }
    await mount((action) => {
      if (action === "open_local") {
        void withBusyState(openLocalFileByDialog);
      }
      if (action === "open_url") {
        urlInputController.requestOpenUrlInput();
      }
    }, getPlaybackSnapshot);
  });

  onBeforeUnmount(() => {
    unmount();
  });

  watch(
    playerHwDecodeMode,
    (mode) => {
      void applyHwDecodePreference(mode);
    },
    { immediate: false },
  );
  watch(
    playerAlwaysOnTop,
    (enabled) => {
      void applyAlwaysOnTopPreference(enabled);
    },
    { immediate: false },
  );
  watch(
    playerVideoScaleMode,
    (mode) => {
      void applyVideoScaleModePreference(mode);
    },
    { immediate: false },
  );

  watch(metadataDurationSeconds, (duration) => {
    if (typeof duration === "number" && Number.isFinite(duration) && duration > 0) {
      void syncPosition(0, duration);
    }
  });

  watch(playbackErrorMessage, (message) => {
    if (message) {
      errorMessage.value = message;
    }
  });

  return {
    playback,
    currentSource,
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
    networkReadBytesPerSecond,
    networkSustainRatio,
    cacheOutputDir: cacheRecordingController.cacheOutputDir,
    errorMessage,
    recordingNoticeMessage,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    firstFrameAtMs,
    latestTelemetry,
    telemetryHistory,
    mediaInfoSnapshot,
    metadataVideoHeight,
    openLocalFileByDialog: () => withBusyState(openLocalFileByDialog),
    openUrl: (url: string) => withBusyState(async () => {
      await urlInputController.submitUrl(url);
    }),
    requestOpenUrlInput: urlInputController.requestOpenUrlInput,
    cancelOpenUrlInput: urlInputController.cancelOpenUrlInput,
    confirmOpenUrlInput: () => withBusyState(urlInputController.confirmOpenUrlInput),
    removeUrlFromPlaylist: urlInputController.removeUrlFromPlaylist,
    clearUrlPlaylist: urlInputController.clearUrlPlaylist,
    play: () => withBusyState(play),
    pause: () => withBusyState(pause),
    stop: () => withBusyState(stop),
    seek: (seconds: number) => withBusyState(() => seek(seconds)),
    seekPreview: (seconds: number) => seekPreview(seconds),
    setRate: (rate: number) => withBusyState(() => setRate(rate)),
    setVolume: (volume: number) => withBusyState(() => setVolume(volume)),
    setMuted: (muted: boolean) => withBusyState(() => setMuted(muted)),
    setQuality: (mode: PlaybackQualityMode) => withBusyState(() => setQuality(mode)),
    toggleCacheRecording: () => withBusyState(cacheRecordingController.toggleCacheRecording),
    requestPreviewFrame: (positionSeconds: number, maxWidth?: number, maxHeight?: number) =>
      requestPreviewFrame(positionSeconds, maxWidth, maxHeight),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      withBusyState(() => syncPosition(positionSeconds, durationSeconds)),
  };
}

function logSeekDecision(
  action: "seek" | "seekPreview",
  positionSeconds: number,
  forceRender: boolean,
  status: string,
) {
  if (!DEV_SEEK_LOG) {
    return;
  }
  const seconds = Number.isFinite(positionSeconds) ? positionSeconds.toFixed(3) : String(positionSeconds);
  console.debug(
    `[media-seek] action=${action} status=${status} target=${seconds}s forceRender=${forceRender}`,
  );
}
