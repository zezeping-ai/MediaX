import { computed, ref } from "vue";
import type { UrlPlaylistItem } from "@/pages/home/composables/usePlaybackPlaylist/types";

type UseMediaUrlInputControllerOptions = {
  openUrl: (url: string) => Promise<void>;
  urlPlaylist: { value: UrlPlaylistItem[] };
  removeUrlFromHistory: (url: string) => void;
  clearUrlHistory: () => void;
};

export function useMediaUrlInputController(options: UseMediaUrlInputControllerOptions) {
  const urlInputValue = ref("");
  const urlDialogVisible = ref(false);

  async function submitUrl(url: string) {
    const normalized = normalizePlayableUrl(url);
    if (!normalized) {
      throw new Error("请输入有效的媒体直链，支持 http(s)、rtsp、rtmp、mms");
    }
    await options.openUrl(normalized);
    return normalized;
  }

  function requestOpenUrlInput() {
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

  return {
    cancelOpenUrlInput,
    clearUrlPlaylist: options.clearUrlHistory,
    confirmOpenUrlInput,
    removeUrlFromPlaylist: options.removeUrlFromHistory,
    requestOpenUrlInput,
    submitUrl,
    urlDialogVisible,
    urlInputValue,
    urlPlaylist: computed(() => options.urlPlaylist.value),
  };
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
