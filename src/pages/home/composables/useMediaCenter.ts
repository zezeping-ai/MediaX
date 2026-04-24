import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { usePreferences } from "@/modules/preferences";
import {
  getMediaSnapshot,
  openMedia,
  pauseMedia,
  playMedia,
  previewMediaFrame,
  seekMedia,
  setMediaMuted,
  setMediaHwDecodeMode,
  setMediaRate,
  setMediaVolume,
  stopMedia,
  syncMediaPosition,
} from "@/modules/media-player";
import type { MediaSnapshot, PreviewFrame } from "@/modules/media-types";

const MEDIA_STATE_EVENT = "media://state";
const MEDIA_MENU_EVENT = "media://menu-action";
const DEV_SEEK_LOG = import.meta.env.DEV;
const MEDIA_ERROR_TEXT: Record<string, string> = {
  INVALID_URL: "媒体地址无效，请检查 URL 或文件路径。",
  OPEN_FAILED: "媒体打开失败，请确认文件存在且格式受支持。",
  STREAM_START_FAILED: "媒体流启动失败，请重试或切换解码源。",
  DECODE_FAILED: "媒体解码失败，请检查媒体格式或尝试转码后播放。",
  UNSUPPORTED_FORMAT: "当前媒体格式暂不支持，请尝试转码后再播放。",
  NETWORK_ERROR: "网络连接异常，请检查网络状态后重试。",
  DECODE_ERROR: "媒体解码失败，可能是编码参数不兼容。",
  INTERNAL_ERROR: "播放器内部错误，请稍后重试。",
};

export function useMediaCenter() {
  const { playerHwDecodeEnabled } = usePreferences();
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const isBusy = ref(false);
  const errorMessage = ref("");
  const lastSyncedSecond = ref(-1);
  const urlInputValue = ref("");
  const urlDialogVisible = ref(false);

  let unlistenMediaEvent: UnlistenFn | null = null;
  let unlistenMenuEvent: UnlistenFn | null = null;
  let snapshotPollingTimer: number | null = null;

  const playback = computed(() => snapshot.value?.playback ?? null);

  async function runPlaybackCommand(command: () => Promise<MediaSnapshot>) {
    snapshot.value = await command();
    return snapshot.value;
  }

  async function refreshSnapshot() {
    snapshot.value = await getMediaSnapshot();
  }

  async function applyHwDecodePreference(enabled: boolean) {
    const mode = enabled ? "auto" : "off";
    try {
      snapshot.value = await setMediaHwDecodeMode(mode);
    } catch {
      // Keep silent here; player surface already emits error events.
    }
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
    await runPlaybackCommand(() => openMedia(path));
    await runPlaybackCommand(playMedia);
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
    await runPlaybackCommand(playMedia);
  }

  async function pause() {
    await runPlaybackCommand(pauseMedia);
  }

  async function stop() {
    await runPlaybackCommand(stopMedia);
  }

  async function seek(positionSeconds: number) {
    const status = playback.value?.status ?? "unknown";
    const forceRender = status === "paused";
    logSeekDecision("seek", positionSeconds, forceRender, status);
    await runPlaybackCommand(() => seekMedia(positionSeconds, { forceRender }));
  }

  async function seekPreview(positionSeconds: number) {
    // Scrubbing preview should stay responsive and not lock controls with busy state.
    try {
      const status = playback.value?.status ?? "unknown";
      logSeekDecision("seekPreview", positionSeconds, false, status);
      await runPlaybackCommand(() => seekMedia(positionSeconds, { forceRender: false }));
    } catch (error) {
      errorMessage.value = toUserErrorMessage(error);
    }
  }

  async function setRate(playbackRate: number) {
    await runPlaybackCommand(() => setMediaRate(playbackRate));
  }

  async function setVolume(volume: number) {
    await runPlaybackCommand(() => setMediaVolume(volume));
  }

  async function setMuted(muted: boolean) {
    await runPlaybackCommand(() => setMediaMuted(muted));
  }

  async function requestPreviewFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
    try {
      return await previewMediaFrame(positionSeconds, maxWidth, maxHeight);
    } catch {
      return null as PreviewFrame | null;
    }
  }

  async function syncPosition(positionSeconds: number, durationSeconds: number) {
    const second = Math.floor(positionSeconds);
    if (second === lastSyncedSecond.value) {
      return;
    }
    lastSyncedSecond.value = second;
    await runPlaybackCommand(() => syncMediaPosition(positionSeconds, durationSeconds));
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
    unlistenMediaEvent = await listen<MediaSnapshot>(MEDIA_STATE_EVENT, (event) => {
      snapshot.value = event.payload;
    });
    unlistenMenuEvent = await listen<string>(MEDIA_MENU_EVENT, (event) => {
      if (event.payload === "open_local") {
        void withBusyState(openLocalFileByDialog);
      }
      if (event.payload === "open_url") {
        requestOpenUrlInput();
      }
    });
    snapshotPollingTimer = window.setInterval(() => {
      void refreshSnapshot();
    }, 1000);
  });

  onBeforeUnmount(() => {
    unlistenMediaEvent?.();
    unlistenMenuEvent?.();
    if (snapshotPollingTimer !== null) {
      window.clearInterval(snapshotPollingTimer);
      snapshotPollingTimer = null;
    }
  });

  watch(playback, (value) => {
    const currentPath = value?.current_path ?? "";
    if (!currentPath) {
      currentSource.value = "";
      return;
    }
    currentSource.value = currentPath;
  });

  watch(
    playerHwDecodeEnabled,
    (enabled) => {
      void applyHwDecodePreference(enabled);
    },
    { immediate: false },
  );

  return {
    playback,
    currentSource,
    urlInputValue,
    urlDialogVisible,
    isBusy,
    errorMessage,
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

function toUserErrorMessage(error: unknown) {
  const rawMessage = error instanceof Error ? error.message : String(error);
  const normalized = rawMessage.trim();
  const [codeCandidate, detailCandidate] = normalized.split(":");
  const code = codeCandidate?.trim().toUpperCase();
  if (code && MEDIA_ERROR_TEXT[code]) {
    const detail = detailCandidate?.trim();
    return detail ? `${MEDIA_ERROR_TEXT[code]}（${detail}）` : MEDIA_ERROR_TEXT[code];
  }
  if (/url|uri|协议|protocol/i.test(normalized)) {
    return MEDIA_ERROR_TEXT.INVALID_URL;
  }
  if (/network|timeout|连接|dns|socket/i.test(normalized)) {
    return MEDIA_ERROR_TEXT.NETWORK_ERROR;
  }
  if (/decode|codec|demux|parse/i.test(normalized)) {
    return MEDIA_ERROR_TEXT.DECODE_ERROR;
  }
  return normalized || MEDIA_ERROR_TEXT.INTERNAL_ERROR;
}

