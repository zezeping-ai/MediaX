import { open } from "@tauri-apps/plugin-dialog";
import type { MediaSnapshot, PlaybackChannelRouting, PlaybackQualityMode } from "@/modules/media-types";

const DEV_SEEK_LOG = import.meta.env.DEV;
const SEEK_COALESCE_WINDOW_MS = 90;

type CreatePlaybackCommandRunnerOptions = {
  commands: {
    openPath: (path: string) => Promise<MediaSnapshot>;
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
    const selected = await open({
      title: "选择本地媒体文件",
      multiple: false,
      filters: [
        {
          name: "Media",
          extensions: [
            "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v", "wmv", "mpeg", "mpg", "ts", "m2ts",
            "mts", "mxf", "rm", "rmvb", "3gp", "3g2", "ogv", "asf", "vob", "f4v", "divx",
            "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "aif", "aiff", "ape",
            "alac", "amr", "ac3", "dts", "mp2", "mka",
          ],
        },
      ],
    });
    if (!selected || Array.isArray(selected)) {
      return null;
    }
    return selected;
  }

  async function openPath(path: string) {
    options.pendingSource.value = path;
    try {
      await run(() => options.commands.openPath(path));
      await run(options.commands.play);
      await options.refreshCacheRecordingStatus();
      options.recordingNoticeMessage.value = "";
      options.errorMessage.value = "";
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
