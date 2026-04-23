import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  getMediaSnapshot,
  openMedia,
  pauseMedia,
  playMedia,
  seekMedia,
  setMediaRate,
  stopMedia,
  syncMediaPosition,
  type MediaSnapshot,
} from "../../../modules/media";

const MEDIA_STATE_EVENT = "media://state";
const MEDIA_MENU_EVENT = "media://menu-action";

export function useMediaCenter() {
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

  async function refreshSnapshot() {
    snapshot.value = await getMediaSnapshot();
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
    snapshot.value = await openMedia(path);
    errorMessage.value = "";
  }

  async function openUrl(url: string) {
    const normalized = url.trim();
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
    snapshot.value = await playMedia();
  }

  async function pause() {
    snapshot.value = await pauseMedia();
  }

  async function stop() {
    snapshot.value = await stopMedia();
  }

  async function seek(positionSeconds: number) {
    snapshot.value = await seekMedia(positionSeconds);
  }

  async function setRate(playbackRate: number) {
    snapshot.value = await setMediaRate(playbackRate);
  }

  async function syncPosition(positionSeconds: number, durationSeconds: number) {
    const second = Math.floor(positionSeconds);
    if (second === lastSyncedSecond.value) {
      return;
    }
    lastSyncedSecond.value = second;
    snapshot.value = await syncMediaPosition(positionSeconds, durationSeconds);
  }

  async function withBusyState(action: () => Promise<void>) {
    isBusy.value = true;
    try {
      await action();
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : String(error);
    } finally {
      isBusy.value = false;
    }
  }

  onMounted(async () => {
    await withBusyState(refreshSnapshot);
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
    currentSource.value = isRemoteSource(currentPath) ? currentPath : convertFileSrc(currentPath);
  });

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
    setRate: (rate: number) => withBusyState(() => setRate(rate)),
    syncPosition: (positionSeconds: number, durationSeconds: number) =>
      withBusyState(() => syncPosition(positionSeconds, durationSeconds)),
  };
}

function isRemoteSource(source: string) {
  return source.startsWith("http://") || source.startsWith("https://");
}

