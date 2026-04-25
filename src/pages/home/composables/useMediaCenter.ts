import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useStorage } from "@vueuse/core";
import { usePreferences } from "@/modules/preferences";
import { getPlaybackSnapshot } from "@/modules/media-player";
import type { MediaSnapshot, PlaybackQualityMode } from "@/modules/media-types";
import { useMediaCommands } from "./useMediaCommands";
import { useMediaErrorMap } from "./useMediaErrorMap";
import { usePlaybackSettings } from "./usePlaybackSettings";
import { useMediaSession } from "./useMediaSession";
const DEV_SEEK_LOG = import.meta.env.DEV;
const URL_PLAYLIST_STORAGE_KEY = "mediax:open-url-playlist";
const MAX_URL_PLAYLIST_SIZE = 50;

type UrlPlaylistItem = {
  url: string;
  lastOpenedAt: number;
};

export function useMediaCenter() {
  const { playerHwDecodeEnabled, playerAlwaysOnTop, playerVideoScaleMode } = usePreferences();
  const {
    snapshot,
    currentSource,
    debugSnapshot,
    debugTimeline,
    networkReadBytesPerSecond,
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
  const urlInputValue = ref("");
  const urlDialogVisible = ref(false);
  const urlPlaylist = useStorage<UrlPlaylistItem[]>(URL_PLAYLIST_STORAGE_KEY, [], localStorage);
  const cacheRecording = ref(false);
  const cacheOutputPath = ref("");
  const cacheFinalizedOutputPath = ref("");
  const cacheOutputDir = ref("");
  const cacheOutputSizeBytes = ref<number | null>(null);
  const cacheWriteSpeedBytesPerSecond = ref<number | null>(null);
  const cacheRecordingStartedAtMs = ref<number | null>(null);
  const cacheRecordingStartPositionSeconds = ref<number | null>(null);
  const recordingNowMs = ref(Date.now());
  const recordingClockHandle = ref<number | null>(null);
  const cacheStatusPollHandle = ref<number | null>(null);
  const cacheSizeSampleBytes = ref<number | null>(null);
  const cacheSizeSampleAtMs = ref<number | null>(null);

  const playback = computed(() => snapshot.value?.playback ?? null);

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
    await refreshCacheRecordingStatus();
    recordingNoticeMessage.value = "";
    errorMessage.value = "";
  }

  async function openUrl(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      throw new Error("请输入有效的播放 URL");
    }
    await openPath(normalized);
    addUrlToPlaylist(normalized);
  }

  function sortedUrlPlaylist(items: UrlPlaylistItem[]) {
    return [...items].sort((a, b) => b.lastOpenedAt - a.lastOpenedAt);
  }

  function sanitizedUrlPlaylist(items: UrlPlaylistItem[]) {
    const dedupedByUrl = new Map<string, UrlPlaylistItem>();
    for (const item of items) {
      const normalized = normalizePlayableUrl(item.url);
      if (!normalized) {
        continue;
      }
      const prev = dedupedByUrl.get(normalized);
      const lastOpenedAt =
        typeof item.lastOpenedAt === "number" && Number.isFinite(item.lastOpenedAt)
          ? item.lastOpenedAt
          : 0;
      if (!prev || lastOpenedAt >= prev.lastOpenedAt) {
        dedupedByUrl.set(normalized, {
          url: normalized,
          lastOpenedAt,
        });
      }
    }
    return sortedUrlPlaylist(Array.from(dedupedByUrl.values())).slice(0, MAX_URL_PLAYLIST_SIZE);
  }

  function addUrlToPlaylist(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      return;
    }
    const now = Date.now();
    const next = sanitizedUrlPlaylist([
      { url: normalized, lastOpenedAt: now },
      ...urlPlaylist.value,
    ]);
    urlPlaylist.value = next;
  }

  function removeUrlFromPlaylist(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      return;
    }
    urlPlaylist.value = urlPlaylist.value.filter((item) => item.url !== normalized);
  }

  function clearUrlPlaylist() {
    urlPlaylist.value = [];
  }

  function requestOpenUrlInput() {
    urlPlaylist.value = sanitizedUrlPlaylist(urlPlaylist.value);
    urlInputValue.value = "";
    urlDialogVisible.value = true;
  }

  function cancelOpenUrlInput() {
    urlDialogVisible.value = false;
  }

  async function confirmOpenUrlInput() {
    const normalized = normalizePlayableUrl(urlInputValue.value);
    if (!normalized) {
      throw new Error("请输入有效的播放 URL");
    }
    urlInputValue.value = normalized;
    await openUrl(normalized);
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
    await refreshCacheRecordingStatus();
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

  async function refreshCacheRecordingStatus() {
    const status = await commands.getCacheRecordingStatus();
    cacheRecording.value = status.recording;
    cacheOutputPath.value = status.output_path ?? "";
    cacheFinalizedOutputPath.value = status.finalized_output_path ?? "";
    cacheOutputSizeBytes.value =
      typeof status.output_size_bytes === "number" && Number.isFinite(status.output_size_bytes)
        ? Math.max(0, status.output_size_bytes)
        : null;
    if (!status.recording) {
      cacheWriteSpeedBytesPerSecond.value = null;
      cacheSizeSampleBytes.value = null;
      cacheSizeSampleAtMs.value = null;
    } else if (cacheOutputSizeBytes.value !== null) {
      const now = Date.now();
      const prevSize = cacheSizeSampleBytes.value;
      const prevAt = cacheSizeSampleAtMs.value;
      if (prevSize !== null && prevAt !== null) {
        const dtSeconds = Math.max((now - prevAt) / 1000, 1e-3);
        const deltaBytes = cacheOutputSizeBytes.value - prevSize;
        cacheWriteSpeedBytesPerSecond.value = Math.max(0, deltaBytes / dtSeconds);
      }
      cacheSizeSampleBytes.value = cacheOutputSizeBytes.value;
      cacheSizeSampleAtMs.value = now;
    }
    cacheRecordingStartedAtMs.value = status.started_at_ms ?? null;
    if (!cacheRecording.value) {
      cacheRecordingStartPositionSeconds.value = null;
    }
    if (!cacheOutputDir.value && cacheOutputPath.value) {
      const idx = cacheOutputPath.value.lastIndexOf("/");
      if (idx > 0) {
        cacheOutputDir.value = cacheOutputPath.value.slice(0, idx);
      }
    }
    applyCacheStatusMessage(status.error_message);
  }

  function applyCacheStatusMessage(message?: string | null) {
    if (!message) {
      return;
    }
    if (message.includes("录制已自动停止")) {
      recordingNoticeMessage.value = message;
      return;
    }
    errorMessage.value = message;
  }

  function startRecordingClock() {
    if (recordingClockHandle.value !== null) {
      return;
    }
    recordingNowMs.value = Date.now();
    recordingClockHandle.value = window.setInterval(() => {
      recordingNowMs.value = Date.now();
    }, 500);
  }

  function stopRecordingClock() {
    if (recordingClockHandle.value === null) {
      return;
    }
    window.clearInterval(recordingClockHandle.value);
    recordingClockHandle.value = null;
  }

  function startCacheStatusPoll() {
    if (cacheStatusPollHandle.value !== null) {
      return;
    }
    cacheStatusPollHandle.value = window.setInterval(() => {
      void refreshCacheRecordingStatus();
    }, 2000);
  }

  function stopCacheStatusPoll() {
    if (cacheStatusPollHandle.value === null) {
      return;
    }
    window.clearInterval(cacheStatusPollHandle.value);
    cacheStatusPollHandle.value = null;
  }

  const cacheRecordingElapsedSeconds = computed(() => {
    if (!cacheRecording.value) {
      return 0;
    }
    const startedAt = cacheRecordingStartedAtMs.value ?? 0;
    if (!Number.isFinite(startedAt) || startedAt <= 0) {
      return 0;
    }
    return Math.max(0, (recordingNowMs.value - startedAt) / 1000);
  });

  const effectiveDurationSeconds = computed(() => {
    const base = playback.value?.duration_seconds ?? metadataDurationSeconds.value ?? 0;
    const baseDuration = Number.isFinite(base) ? Math.max(0, base) : 0;
    if (!cacheRecording.value) {
      return baseDuration;
    }
    const source = currentSource.value;
    const isLiveM3u8 = /\.m3u8(\?|#|$)/i.test(source);
    const startPosition = cacheRecordingStartPositionSeconds.value ?? 0;
    const aligned = Math.max(0, startPosition) + cacheRecordingElapsedSeconds.value;
    // When watching the live source, keep duration aligned to live timeline to avoid
    // "current time > total duration" after recording starts later.
    // In time-shift playback (local mp4), duration should start from 0 at recording start.
    const override = isLiveM3u8 ? aligned : cacheRecordingElapsedSeconds.value;
    return Math.max(baseDuration, override);
  });

  async function toggleCacheRecording() {
    if (cacheRecording.value) {
      const stopped = await commands.stopCacheRecording();
      cacheRecording.value = stopped.recording;
      cacheOutputPath.value = stopped.output_path ?? "";
      cacheFinalizedOutputPath.value =
        stopped.finalized_output_path ?? stopped.output_path ?? "";
      cacheRecordingStartedAtMs.value = stopped.started_at_ms ?? null;
      cacheRecordingStartPositionSeconds.value = null;
      stopRecordingClock();
      applyCacheStatusMessage(stopped.error_message);
      return;
    }
    const selected = await open({
      title: "选择缓存输出目录",
      directory: true,
      multiple: false,
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }
    cacheOutputDir.value = String(selected);
    const started = await commands.startCacheRecording(selected);
    cacheRecording.value = started.recording;
    cacheOutputPath.value = started.output_path ?? "";
    cacheFinalizedOutputPath.value = "";
    cacheOutputSizeBytes.value = started.output_size_bytes ?? 0;
    cacheWriteSpeedBytesPerSecond.value = null;
    cacheSizeSampleBytes.value =
      typeof started.output_size_bytes === "number" && Number.isFinite(started.output_size_bytes)
        ? Math.max(0, started.output_size_bytes)
        : 0;
    cacheSizeSampleAtMs.value = Date.now();
    cacheRecordingStartedAtMs.value = started.started_at_ms ?? null;
    cacheRecordingStartPositionSeconds.value = playback.value?.position_seconds ?? 0;
    if (cacheRecording.value) {
      startRecordingClock();
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
    await refreshCacheRecordingStatus();
    if (cacheRecording.value) {
      startRecordingClock();
      startCacheStatusPoll();
    }
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
    if (cacheRecording.value) {
      void commands.stopCacheRecording();
    }
    stopRecordingClock();
    stopCacheStatusPoll();
    unmount();
  });

  watch(cacheRecording, (recording) => {
    if (recording) {
      startCacheStatusPoll();
      return;
    }
    stopCacheStatusPoll();
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
    effectiveDurationSeconds,
    urlInputValue,
    urlDialogVisible,
    urlPlaylist: computed(() => sortedUrlPlaylist(urlPlaylist.value)),
    isBusy,
    cacheRecording,
    cacheOutputPath,
    cacheFinalizedOutputPath,
    cacheOutputSizeBytes,
    cacheWriteSpeedBytesPerSecond,
    networkReadBytesPerSecond,
    cacheOutputDir,
    errorMessage,
    recordingNoticeMessage,
    debugSnapshot,
    debugTimeline,
    mediaInfoSnapshot,
    metadataVideoHeight,
    openLocalFileByDialog: () => withBusyState(openLocalFileByDialog),
    openUrl: (url: string) => withBusyState(() => openUrl(url)),
    requestOpenUrlInput,
    cancelOpenUrlInput,
    confirmOpenUrlInput: () => withBusyState(confirmOpenUrlInput),
    removeUrlFromPlaylist,
    clearUrlPlaylist,
    play: () => withBusyState(play),
    pause: () => withBusyState(pause),
    stop: () => withBusyState(stop),
    seek: (seconds: number) => withBusyState(() => seek(seconds)),
    seekPreview: (seconds: number) => seekPreview(seconds),
    setRate: (rate: number) => withBusyState(() => setRate(rate)),
    setVolume: (volume: number) => withBusyState(() => setVolume(volume)),
    setMuted: (muted: boolean) => withBusyState(() => setMuted(muted)),
    setQuality: (mode: PlaybackQualityMode) => withBusyState(() => setQuality(mode)),
    toggleCacheRecording: () => withBusyState(toggleCacheRecording),
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

