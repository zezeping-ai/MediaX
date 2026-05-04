import { open } from "@tauri-apps/plugin-dialog";
import { normalizeLocalPlaybackPath } from "@/modules/local-file-path";
import { playbackPickLocalFile } from "@/modules/media-player";
import type { MediaSnapshot, PlaybackChannelRouting, PlaybackQualityMode } from "@/modules/media-types";

const DEV_SEEK_LOG = import.meta.env.DEV;

type CreatePlaybackCommandRunnerOptions = {
  commands: {
    openSource: (source: string) => Promise<MediaSnapshot>;
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
  recordingNoticeMessage: { value: string };
  lastSyncedSecond: { value: number };
  captureError: (error: unknown) => void;
  updateSnapshot: (snapshot: MediaSnapshot) => void;
  refreshCacheRecordingStatus: () => Promise<void>;
};

export function createPlaybackCommandRunner(options: CreatePlaybackCommandRunnerOptions) {
  async function runSnapshotCommand(command: () => Promise<MediaSnapshot>) {
    const nextSnapshot = await command();
    options.updateSnapshot(nextSnapshot);
    return nextSnapshot;
  }

  async function runGuarded(action: () => Promise<void>) {
    try {
      await action();
    } catch (error) {
      options.captureError(error);
    }
  }

  async function withPendingSource<T>(source: string, action: () => Promise<T>) {
    options.pendingSource.value = source;
    try {
      return await action();
    } finally {
      options.pendingSource.value = "";
    }
  }

  async function finalizeSourceOpen() {
    await options.refreshCacheRecordingStatus();
    options.recordingNoticeMessage.value = "";
  }

  function getPlaybackStatus() {
    return options.playback.value?.status ?? "unknown";
  }

  function shouldSyncPosition(positionSeconds: number) {
    const second = Math.floor(positionSeconds);
    if (second === options.lastSyncedSecond.value) {
      return false;
    }
    options.lastSyncedSecond.value = second;
    return true;
  }

  async function pickLocalPathViaPluginDialog(): Promise<string | null> {
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
    const path = coerceDialogFilePath(selected);
    return path ? normalizeLocalPlaybackPath(path) : null;
  }

  async function openLocalFileByDialog() {
    const usedNativePicker = await playbackPickLocalFile();
    if (usedNativePicker) {
      return null;
    }
    return pickLocalPathViaPluginDialog();
  }

  async function openSource(source: string) {
    await withPendingSource(source, async () => {
      await runSnapshotCommand(() => options.commands.openSource(source));
      const playSnapshot = await runSnapshotCommand(options.commands.play);
      // Occasionally the first resume after source switch can race with runtime teardown/startup.
      // If snapshot still isn't playing, issue one immediate replay as a safety net.
      if (playSnapshot.playback?.status !== "playing") {
        await runSnapshotCommand(options.commands.play);
      }
      await finalizeSourceOpen();
    });
  }

  async function play() {
    await runSnapshotCommand(options.commands.play);
  }

  async function pause() {
    await runSnapshotCommand(options.commands.pause);
  }

  async function stop() {
    await runSnapshotCommand(options.commands.stop);
    await options.refreshCacheRecordingStatus();
  }

  async function seek(positionSeconds: number) {
    const status = getPlaybackStatus();
    const forceRender = status === "paused";
    logSeekDecision("seek", positionSeconds, forceRender, status);
    await runSnapshotCommand(() => options.commands.seek(positionSeconds, forceRender));
  }

  async function seekPreview(positionSeconds: number) {
    await runGuarded(async () => {
      const status = getPlaybackStatus();
      logSeekDecision("seekPreview", positionSeconds, false, status);
      await runSnapshotCommand(() => options.commands.seek(positionSeconds, false));
    });
  }

  async function setRate(playbackRate: number) {
    await runSnapshotCommand(() => options.commands.setRate(playbackRate));
  }

  async function setVolume(volume: number) {
    await runSnapshotCommand(() => options.commands.setVolume(volume));
  }

  async function setMuted(muted: boolean) {
    await runSnapshotCommand(() => options.commands.setMuted(muted));
  }

  async function setLeftChannelVolume(volume: number) {
    await runSnapshotCommand(() => options.commands.setLeftChannelVolume(volume));
  }

  async function setRightChannelVolume(volume: number) {
    await runSnapshotCommand(() => options.commands.setRightChannelVolume(volume));
  }

  async function setLeftChannelMuted(muted: boolean) {
    await runSnapshotCommand(() => options.commands.setLeftChannelMuted(muted));
  }

  async function setRightChannelMuted(muted: boolean) {
    await runSnapshotCommand(() => options.commands.setRightChannelMuted(muted));
  }

  async function setChannelRouting(routing: PlaybackChannelRouting) {
    await runSnapshotCommand(() => options.commands.setChannelRouting(routing));
  }

  async function setQuality(mode: PlaybackQualityMode) {
    await runSnapshotCommand(() => options.commands.setQuality(mode));
  }

  async function syncPosition(positionSeconds: number, durationSeconds: number) {
    if (!shouldSyncPosition(positionSeconds)) {
      return;
    }
    await runSnapshotCommand(() => options.commands.syncPosition(positionSeconds, durationSeconds));
  }

  return {
    openLocalFileByDialog,
    openSource,
    pause,
    play,
    run: runSnapshotCommand,
    runGuarded,
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

function coerceDialogFilePath(selected: unknown): string | null {
  if (selected == null) {
    return null;
  }
  if (typeof selected === "string") {
    return selected;
  }
  if (Array.isArray(selected)) {
    const first = selected[0];
    if (typeof first === "string") {
      return first;
    }
    if (first && typeof first === "object" && "path" in first) {
      const p = (first as { path?: unknown }).path;
      return typeof p === "string" ? p : null;
    }
    return null;
  }
  if (typeof selected === "object" && "path" in selected) {
    const p = (selected as { path?: unknown }).path;
    return typeof p === "string" ? p : null;
  }
  return null;
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
