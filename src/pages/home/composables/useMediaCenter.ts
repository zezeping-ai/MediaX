import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { usePreferences } from "@/modules/preferences";
import { getPlaybackSnapshot } from "@/modules/media-player";
import type { MediaSnapshot } from "@/modules/media-types";
import { useMediaCommands } from "./useMediaCommands";
import { useMediaErrorMap } from "./useMediaErrorMap";
import { usePlaybackSettings } from "./usePlaybackSettings";
import { useMediaSession } from "./useMediaSession";
const DEV_SEEK_LOG = import.meta.env.DEV;

export function useMediaCenter() {
  const { playerHwDecodeEnabled, playerAlwaysOnTop, playerVideoScaleMode } = usePreferences();
  const {
    snapshot,
    currentSource,
    debugSnapshot,
    metadataDurationSeconds,
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
  const lastSyncedSecond = ref(-1);
  const urlInputValue = ref("");
  const urlDialogVisible = ref(false);

  const playback = computed(() => snapshot.value?.playback ?? null);

  async function runPlaybackCommand(command: () => Promise<MediaSnapshot>) {
    const next = await command();
    updateSnapshot(next);
    return next;
  }

  async function refreshSnapshot() {
    updateSnapshot(await getPlaybackSnapshot());
  }

  async function applyHwDecodePreference(enabled: boolean) {
    const next = await playbackSettings.applyHwDecode(enabled);
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
    await runPlaybackCommand(() => commands.openPath(path));
    await runPlaybackCommand(commands.play);
    errorMessage.value = "";
  }

  async function openUrl(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      throw new Error("请输入有效的播放 URL");
    }
    await openPath(normalized);
  }

  function requestOpenUrlInput() {
    urlDialogVisible.value = true;
  }

  function cancelOpenUrlInput() {
    urlDialogVisible.value = false;
  }

  async function confirmOpenUrlInput() {
    await openUrl(urlInputValue.value);
    urlDialogVisible.value = false;
  }

  async function play() {
    await runPlaybackCommand(commands.play);
  }

  async function pause() {
    await runPlaybackCommand(commands.pause);
  }

  async function stop() {
    await runPlaybackCommand(commands.stop);
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
    await applyHwDecodePreference(playerHwDecodeEnabled.value);
    await applyAlwaysOnTopPreference(playerAlwaysOnTop.value);
    await applyVideoScaleModePreference(playerVideoScaleMode.value);
    await mount((action) => {
      if (action === "open_local") {
        void withBusyState(openLocalFileByDialog);
      }
      if (action === "open_url") {
        requestOpenUrlInput();
      }
    }, getPlaybackSnapshot);
  });

  onBeforeUnmount(() => {
    unmount();
  });

  watch(
    playerHwDecodeEnabled,
    (enabled) => {
      void applyHwDecodePreference(enabled);
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
    urlInputValue,
    urlDialogVisible,
    isBusy,
    errorMessage,
    debugSnapshot,
    openLocalFileByDialog: () => withBusyState(openLocalFileByDialog),
    openUrl: (url: string) => withBusyState(() => openUrl(url)),
    requestOpenUrlInput,
    cancelOpenUrlInput,
    confirmOpenUrlInput: () => withBusyState(confirmOpenUrlInput),
    play: () => withBusyState(play),
    pause: () => withBusyState(pause),
    stop: () => withBusyState(stop),
    seek: (seconds: number) => withBusyState(() => seek(seconds)),
    seekPreview: (seconds: number) => seekPreview(seconds),
    setRate: (rate: number) => withBusyState(() => setRate(rate)),
    setVolume: (volume: number) => withBusyState(() => setVolume(volume)),
    setMuted: (muted: boolean) => withBusyState(() => setMuted(muted)),
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

function normalizePlayableUrl(raw: string) {
  const value = raw.trim();
  if (!value) {
    return "";
  }

  // Accept URLs without explicit scheme, default to https.
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`;
  let parsed: URL;
  try {
    parsed = new URL(withScheme);
  } catch {
    return "";
  }

  // Keep supported streaming protocols explicit to avoid treating local paths as URLs.
  if (!/^(https?|rtsp|rtmp|mms):$/i.test(parsed.protocol)) {
    return "";
  }
  return parsed.toString();
}

