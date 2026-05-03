import type {
  PlaybackChannelRouting,
  PlaybackQualityMode,
  PreviewFrame,
} from "@/modules/media-types";
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
};

export function createMediaCenterActions(options: CreateMediaCenterActionsOptions) {
  const {
    cacheRecordingController,
    playbackRunner,
    requestPreviewFrame,
    urlInputController,
    withBusyState,
  } = options;

  function runBlockingAction(action: () => Promise<void>) {
    return withBusyState(action);
  }

  function runRealtimeAction(action: () => Promise<void>) {
    return playbackRunner.runGuarded(action);
  }

  function runStartupAction(action: () => Promise<void>) {
    return runBlockingAction(action);
  }

  return {
    openSource: (source: string) => runStartupAction(async () => {
      await playbackRunner.openSource(source);
    }),
    openLocalFileByDialog: () => runStartupAction(async () => {
      const selectedPath = await playbackRunner.openLocalFileByDialog();
      if (selectedPath) {
        await playbackRunner.openSource(selectedPath);
      }
    }),
    openUrl: (url: string) => runStartupAction(async () => {
      await urlInputController.submitUrl(url);
    }),
    requestOpenUrlInput: urlInputController.requestOpenUrlInput,
    cancelOpenUrlInput: urlInputController.cancelOpenUrlInput,
    confirmOpenUrlInput: () => runStartupAction(urlInputController.confirmOpenUrlInput),
    removeUrlFromPlaylist: urlInputController.removeUrlFromPlaylist,
    clearUrlPlaylist: urlInputController.clearUrlPlaylist,
    play: () => runBlockingAction(playbackRunner.play),
    pause: () => runBlockingAction(playbackRunner.pause),
    stop: () => runBlockingAction(playbackRunner.stop),
    seek: (seconds: number) => runRealtimeAction(() => playbackRunner.seek(seconds)),
    seekPreview: (seconds: number) => playbackRunner.seekPreview(seconds),
    setRate: (rate: number) => runRealtimeAction(() => playbackRunner.setRate(rate)),
    setVolume: (volume: number) =>
      runRealtimeAction(() => playbackRunner.setVolume(volume)),
    setMuted: (muted: boolean) =>
      runRealtimeAction(() => playbackRunner.setMuted(muted)),
    setLeftChannelVolume: (volume: number) =>
      runRealtimeAction(() => playbackRunner.setLeftChannelVolume(volume)),
    setRightChannelVolume: (volume: number) =>
      runRealtimeAction(() => playbackRunner.setRightChannelVolume(volume)),
    setLeftChannelMuted: (muted: boolean) =>
      runRealtimeAction(() => playbackRunner.setLeftChannelMuted(muted)),
    setRightChannelMuted: (muted: boolean) =>
      runRealtimeAction(() => playbackRunner.setRightChannelMuted(muted)),
    setChannelRouting: (routing: PlaybackChannelRouting) =>
      runRealtimeAction(() => playbackRunner.setChannelRouting(routing)),
    setQuality: (mode: PlaybackQualityMode) => runBlockingAction(() => playbackRunner.setQuality(mode)),
    toggleCacheRecording: () => runBlockingAction(cacheRecordingController.toggleCacheRecording),
    requestPreviewFrame: (positionSeconds: number, maxWidth?: number, maxHeight?: number) =>
      requestPreviewFrame(positionSeconds, maxWidth, maxHeight),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      runRealtimeAction(() => playbackRunner.syncPosition(positionSeconds, durationSeconds)),
  };
}
