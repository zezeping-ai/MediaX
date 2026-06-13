import type {
  PlaybackChannelRouting,
  PlaybackQualityMode,
  PreviewFrame,
} from "@/modules/media-types";
import { scanMediaDirectory } from "@/modules/media-library";
import { open } from "@tauri-apps/plugin-dialog";
import { resolveDialogPath } from "@/modules/resolve-dialog-path";
import type { Ref } from "vue";
import type { PlaybackPlaylistController } from "@/pages/home/composables/usePlaybackPlaylist";
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
  playlistController: PlaybackPlaylistController;
  openPathWithPlaylist: (source: string) => Promise<void>;
  currentSource: Ref<string>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
  resumePromptPositionSeconds: Ref<number | null>;
  onNoticeMessage: (message: string) => void;
};

export function createMediaCenterActions(options: CreateMediaCenterActionsOptions) {
  const {
    cacheRecordingController,
    currentSource,
    openPathWithPlaylist,
    playbackRunner,
    playlistController,
    requestPreviewFrame,
    urlInputController,
    withBusyState,
    onNoticeMessage,
  } = options;

  async function pickDirectory(title: string): Promise<string | null> {
    const selected = await open({
      title,
      directory: true,
      multiple: false,
    });
    return resolveDialogPath(selected);
  }

  async function exportCurrentAudio() {
    const outputDir = await pickDirectory("选择音频导出目录");
    if (!outputDir) {
      return;
    }
    const outputPath = await playbackRunner.exportCurrentAudio(outputDir);
    onNoticeMessage(`音频已导出：${outputPath}`);
  }

  async function importLocalPaths(paths: string[], playFirst?: boolean) {
    if (!paths.length) {
      return 0;
    }
    const shouldPlayFirst = playFirst ?? !currentSource.value.trim();
    const imported = await playlistController.importSources(paths, { playFirst: shouldPlayFirst });
    if (imported > 0) {
      onNoticeMessage(`已加入 ${imported} 个媒体到播放列表`);
    }
    return imported;
  }

  return {
    openPath: (path: string) => withBusyState(async () => {
      await openPathWithPlaylist(path);
    }),
    openLocalFileByDialog: async () => {
      const selectedPath = await playbackRunner.openLocalFileByDialog();
      if (!selectedPath) {
        return;
      }
      await withBusyState(async () => {
        await openPathWithPlaylist(selectedPath);
      });
    },
    importLocalFilesToQueue: async () => {
      const paths = await playbackRunner.pickLocalMediaFiles();
      if (!paths.length) {
        return;
      }
      await withBusyState(async () => {
        await importLocalPaths(paths);
      });
    },
    importFolderToQueue: async () => {
      const directory = await pickDirectory("选择文件夹导入播放列表");
      if (!directory) {
        return;
      }
      await withBusyState(async () => {
        const paths = await scanMediaDirectory(directory);
        if (!paths.length) {
          onNoticeMessage("所选文件夹中没有可播放的媒体文件");
          return;
        }
        await importLocalPaths(paths);
      });
    },
    importPathsToQueue: (paths: string[]) => withBusyState(async () => {
      await importLocalPaths(paths);
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
    seek: (seconds: number) => {
      options.resumePromptPositionSeconds.value = null;
      return playbackRunner.runWithoutBusyLock(() => playbackRunner.seek(seconds));
    },
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
    exportCurrentAudio: () => withBusyState(exportCurrentAudio),
    toggleCacheRecording: () => withBusyState(cacheRecordingController.toggleCacheRecording),
    requestPreviewFrame: (positionSeconds: number, maxWidth?: number, maxHeight?: number) =>
      requestPreviewFrame(positionSeconds, maxWidth, maxHeight),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      withBusyState(() => playbackRunner.syncPosition(positionSeconds, durationSeconds)),
    togglePlaylistPanel: playlistController.togglePanel,
    playNextInQueue: () => withBusyState(async () => {
      await playlistController.tryPlayNextInQueue();
    }),
    playPreviousInQueue: () => withBusyState(async () => {
      await playlistController.tryPlayPreviousInQueue();
    }),
    playQueueItem: (id: string) => withBusyState(async () => {
      await playlistController.playQueueItem(id);
    }),
    playHistoryItem: (id: string) => withBusyState(async () => {
      await playlistController.playHistoryItem(id);
    }),
    removeQueueItem: playlistController.removeFromQueue,
    removeHistoryItem: playlistController.removeFromHistory,
    reorderQueue: playlistController.reorderQueue,
    addToQueue: playlistController.addToQueue,
    clearQueue: playlistController.clearQueue,
    clearHistory: playlistController.clearHistory,
    setAdvanceMode: playlistController.setAdvanceMode,
    handleTrackEnded: playlistController.handleTrackEnded,
    tryPlayNextInQueue: playlistController.tryPlayNextInQueue,
  };
}
