import { open } from "@tauri-apps/plugin-dialog";
import { LOCAL_MEDIA_DIALOG_FILTERS, filterLocalMediaPaths } from "@/modules/local-media-files";
import { getSavedPlaybackPosition } from "@/modules/media-library";
import { resolveDialogPath, resolveDialogPaths } from "@/modules/resolve-dialog-path";
import type { MediaSnapshot, PlaybackChannelRouting, PlaybackQualityMode } from "@/modules/media-types";
import type { Ref } from "vue";

const DEV_SEEK_LOG = import.meta.env.DEV;
const SEEK_COALESCE_WINDOW_MS = 90;
const RESUME_POSITION_EPSILON_SECONDS = 0.5;

type CreatePlaybackCommandRunnerOptions = {
  commands: {
    openPath: (path: string, resumeLastPosition?: boolean) => Promise<MediaSnapshot>;
    play: () => Promise<MediaSnapshot>;
    pause: () => Promise<MediaSnapshot>;
    stop: () => Promise<MediaSnapshot>;
    seek: (positionSeconds: number, forceRender?: boolean) => Promise<MediaSnapshot>;
    setRate: (rate: number) => Promise<MediaSnapshot>;
    setVolume: (volume: number) => Promise<MediaSnapshot>;
    setMuted: (muted: boolean) => Promise<MediaSnapshot>;
    setLeftChannelVolume: (volume: number) => Promise<MediaSnapshot>;
    setRightChannelVolume: (volume: number) => Promise<MediaSnapshot>;
    setLeftChannelMuted: (muted: boolean) => Promise<MediaSnapshot>;
    setRightChannelMuted: (muted: boolean) => Promise<MediaSnapshot>;
    setChannelRouting: (routing: PlaybackChannelRouting) => Promise<MediaSnapshot>;
    setQuality: (mode: PlaybackQualityMode) => Promise<MediaSnapshot>;
    exportCurrentAudio: (outputDir: string) => Promise<string>;
    syncPosition: (positionSeconds: number, durationSeconds: number) => Promise<MediaSnapshot>;
  };
  playback: { value: { status?: string } | null };
  pendingSource: { value: string };
  errorMessage: { value: string };
  recordingNoticeMessage: { value: string };
  lastSyncedSecond: { value: number };
  toUserErrorMessage: (error: unknown) => string;
  updateSnapshot: (snapshot: MediaSnapshot) => void;
  refreshCacheRecordingStatus: () => Promise<void>;
  resumeLastPosition: Ref<boolean>;
  resumePromptPositionSeconds: Ref<number | null>;
};

export function createPlaybackCommandRunner(options: CreatePlaybackCommandRunnerOptions) {
  let seekTimer: number | null = null;
  let seekInFlight: Promise<void> | null = null;
  let seekQueuedTarget: number | null = null;
  let seekQueuedResolver: (() => void) | null = null;
  let seekQueuedRejecter: ((error: unknown) => void) | null = null;

  function flushSeekQueueSoon() {
    if (seekTimer !== null) {
      return;
    }
    seekTimer = window.setTimeout(() => {
      seekTimer = null;
      void runSeekFlush();
    }, SEEK_COALESCE_WINDOW_MS);
  }

  async function runSeekFlush() {
    if (seekInFlight) {
      return;
    }
    if (seekQueuedTarget === null) {
      return;
    }
    const target = seekQueuedTarget;
    seekQueuedTarget = null;
    const resolve = seekQueuedResolver;
    const reject = seekQueuedRejecter;
    seekQueuedResolver = null;
    seekQueuedRejecter = null;

    seekInFlight = (async () => {
      try {
        const status = options.playback.value?.status ?? "unknown";
        const forceRender = status === "paused";
        logSeekDecision("seek", target, forceRender, status);
        await run(() => options.commands.seek(target, forceRender));
        resolve?.();
      } catch (error) {
        reject?.(error);
        options.errorMessage.value = options.toUserErrorMessage(error);
      } finally {
        seekInFlight = null;
        if (seekQueuedTarget !== null) {
          flushSeekQueueSoon();
        }
      }
    })();

    await seekInFlight;
  }

  async function run(command: () => Promise<MediaSnapshot>) {
    const nextSnapshot = await command();
    options.updateSnapshot(nextSnapshot);
    return nextSnapshot;
  }

  async function openLocalFileByDialog() {
    try {
      const selected = await open({
        title: "选择本地媒体文件",
        multiple: false,
        filters: LOCAL_MEDIA_DIALOG_FILTERS,
      });
      const resolved = resolveDialogPath(selected);
      if (resolved) {
        return resolved;
      }
    } catch {
      // Filtered picker can fail on some platforms; fall back to the system file dialog.
    }
    const fallback = await open({
      title: "选择本地媒体文件",
      multiple: false,
    });
    return resolveDialogPath(fallback);
  }

  async function pickLocalMediaFiles() {
    try {
      const selected = await open({
        title: "选择媒体文件加入播放列表",
        multiple: true,
        filters: LOCAL_MEDIA_DIALOG_FILTERS,
      });
      const resolved = filterLocalMediaPaths(resolveDialogPaths(selected));
      if (resolved.length) {
        return resolved;
      }
    } catch {
      // Filtered picker can fail on some platforms; fall back to the system file dialog.
    }
    const fallback = await open({
      title: "选择媒体文件加入播放列表",
      multiple: true,
    });
    return filterLocalMediaPaths(resolveDialogPaths(fallback));
  }

  async function openPath(path: string) {
    options.pendingSource.value = path;
    try {
      let savedPositionSeconds = 0;
      if (options.resumeLastPosition.value) {
        try {
          savedPositionSeconds = await getSavedPlaybackPosition(path);
        } catch {
          savedPositionSeconds = 0;
        }
      }
      await run(() => options.commands.openPath(path, false));
      await run(options.commands.play);
      await options.refreshCacheRecordingStatus();
      options.recordingNoticeMessage.value = "";
      options.errorMessage.value = "";
      if (
        options.resumeLastPosition.value
        && savedPositionSeconds > RESUME_POSITION_EPSILON_SECONDS
      ) {
        options.resumePromptPositionSeconds.value = savedPositionSeconds;
      } else {
        options.resumePromptPositionSeconds.value = null;
      }
    } finally {
      options.pendingSource.value = "";
    }
  }

  async function play() {
    await run(options.commands.play);
  }

  async function pause() {
    await run(options.commands.pause);
  }

  async function stop() {
    options.resumePromptPositionSeconds.value = null;
    await run(options.commands.stop);
    await options.refreshCacheRecordingStatus();
  }

  function seek(positionSeconds: number) {
    seekQueuedTarget = positionSeconds;
    if (!seekQueuedResolver) {
      const promise = new Promise<void>((resolve, reject) => {
        seekQueuedResolver = resolve;
        seekQueuedRejecter = reject;
      });
      flushSeekQueueSoon();
      return promise;
    }
    flushSeekQueueSoon();
    return new Promise<void>((resolve, reject) => {
      const prevResolve = seekQueuedResolver;
      const prevReject = seekQueuedRejecter;
      seekQueuedResolver = () => {
        prevResolve?.();
        resolve();
      };
      seekQueuedRejecter = (error) => {
        prevReject?.(error);
        reject(error);
      };
    });
  }

  async function seekPreview(positionSeconds: number) {
    try {
      const status = options.playback.value?.status ?? "unknown";
      logSeekDecision("seekPreview", positionSeconds, false, status);
      await run(() => options.commands.seek(positionSeconds, false));
    } catch (error) {
      options.errorMessage.value = options.toUserErrorMessage(error);
    }
  }

  async function setRate(playbackRate: number) {
    await run(() => options.commands.setRate(playbackRate));
  }

  async function setVolume(volume: number) {
    await run(() => options.commands.setVolume(volume));
  }

  async function setMuted(muted: boolean) {
    await run(() => options.commands.setMuted(muted));
  }

  async function setLeftChannelVolume(volume: number) {
    await run(() => options.commands.setLeftChannelVolume(volume));
  }

  async function setRightChannelVolume(volume: number) {
    await run(() => options.commands.setRightChannelVolume(volume));
  }

  async function setLeftChannelMuted(muted: boolean) {
    await run(() => options.commands.setLeftChannelMuted(muted));
  }

  async function setRightChannelMuted(muted: boolean) {
    await run(() => options.commands.setRightChannelMuted(muted));
  }

  async function setChannelRouting(routing: PlaybackChannelRouting) {
    await run(() => options.commands.setChannelRouting(routing));
  }

  async function setQuality(mode: PlaybackQualityMode) {
    await run(() => options.commands.setQuality(mode));
  }

  async function syncPosition(positionSeconds: number, durationSeconds: number) {
    const second = Math.floor(positionSeconds);
    if (second === options.lastSyncedSecond.value) {
      return;
    }
    options.lastSyncedSecond.value = second;
    await run(() => options.commands.syncPosition(positionSeconds, durationSeconds));
  }

  async function exportCurrentAudio(outputDir: string) {
    return options.commands.exportCurrentAudio(outputDir);
  }

  async function runWithoutBusyLock(action: () => Promise<void>) {
    try {
      await action();
    } catch (error) {
      options.errorMessage.value = options.toUserErrorMessage(error);
    }
  }

  return {
    openLocalFileByDialog,
    openPath,
    pause,
    pickLocalMediaFiles,
    play,
    run,
    runWithoutBusyLock,
    seek,
    seekPreview,
    setMuted,
    setLeftChannelMuted,
    setLeftChannelVolume,
    setChannelRouting,
    setQuality,
    setRate,
    setRightChannelMuted,
    setRightChannelVolume,
    setVolume,
    stop,
    syncPosition,
    exportCurrentAudio,
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
