import { computed, ref } from "vue";
import { useStorage } from "@vueuse/core";

const URL_PLAYLIST_STORAGE_KEY = "mediax:open-url-playlist";
const MAX_URL_PLAYLIST_SIZE = 50;

export type UrlPlaylistItem = {
  url: string;
  lastOpenedAt: number;
};

type UseMediaUrlInputControllerOptions = {
  openUrl: (url: string) => Promise<void>;
};

export function useMediaUrlInputController(options: UseMediaUrlInputControllerOptions) {
  const urlInputValue = ref("");
  const urlDialogVisible = ref(false);
  const urlPlaylist = useStorage<UrlPlaylistItem[]>(URL_PLAYLIST_STORAGE_KEY, [], localStorage);

  async function submitUrl(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      throw new Error("请输入有效的播放 URL");
    }
    await options.openUrl(normalized);
    addUrlToPlaylist(normalized);
    return normalized;
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
    const normalized = await submitUrl(urlInputValue.value);
    urlInputValue.value = normalized;
    urlDialogVisible.value = false;
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

  return {
    cancelOpenUrlInput,
    clearUrlPlaylist,
    confirmOpenUrlInput,
    removeUrlFromPlaylist,
    requestOpenUrlInput,
    submitUrl,
    urlDialogVisible,
    urlInputValue,
    urlPlaylist: computed(() => sortedUrlPlaylist(urlPlaylist.value)),
  };

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

function normalizePlayableUrl(raw: string) {
  const value = raw.trim();
  if (!value) {
    return "";
  }
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(value) ? value : `https://${value}`;
  let parsed: URL;
  try {
    parsed = new URL(withScheme);
  } catch {
    return "";
  }
  if (!/^(https?|rtsp|rtmp|mms):$/i.test(parsed.protocol)) {
    return "";
  }
  return parsed.toString();
}
