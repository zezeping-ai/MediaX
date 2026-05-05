import type {
  PlaybackChannelRouting,
  PlaybackQualityMode,
  PreviewFrame,
} from "@/modules/media-types";
import { open } from "@tauri-apps/plugin-dialog";
import type { createPlaybackCommandRunner } from "./createPlaybackCommandRunner";
import type { useCacheRecordingController } from "./useCacheRecordingController";
import type { useMediaUrlInputController } from "./useMediaUrlInputController";

type PlaybackRunner = ReturnType<typeof createPlaybackCommandRunner>;
type CacheRecordingController = ReturnType<typeof useCacheRecordingController>;
type UrlInputController = ReturnType<typeof useMediaUrlInputController>;

type CreateMediaCenterActionsOptions = {
  withBusyState: (action: () => Promise<void>) => Promise<void>;
  playbackRunner: PlaybackRunner;
  cacheRecordingController: CacheRecordingController;
  urlInputController: UrlInputController;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
  onNoticeMessage: (message: string) => void;
};

export function createMediaCenterActions(options: CreateMediaCenterActionsOptions) {
  const {
    cacheRecordingController,
    playbackRunner,
    requestPreviewFrame,
    urlInputController,
    withBusyState,
    onNoticeMessage,
  } = options;

  return {
    openPath: (path: string) => withBusyState(async () => {
      await playbackRunner.openPath(path);
    }),
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
    setChannelRouting: (routing: PlaybackChannelRouting) =>
      playbackRunner.runWithoutBusyLock(() => playbackRunner.setChannelRouting(routing)),
    setQuality: (mode: PlaybackQualityMode) => withBusyState(() => playbackRunner.setQuality(mode)),
    exportCurrentAudio: () => withBusyState(async () => {
      const selected = await open({
        title: "选择音频导出目录",
        directory: true,
        multiple: false,
      });
      if (!selected || Array.isArray(selected)) {
        return;
      }
      const outputPath = await playbackRunner.exportCurrentAudio(String(selected));
      onNoticeMessage(`音频已导出：${outputPath}`);
    }),
    toggleCacheRecording: () => withBusyState(cacheRecordingController.toggleCacheRecording),
    requestPreviewFrame: (positionSeconds: number, maxWidth?: number, maxHeight?: number) =>
      requestPreviewFrame(positionSeconds, maxWidth, maxHeight),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      withBusyState(() => playbackRunner.syncPosition(positionSeconds, durationSeconds)),
  };
}
