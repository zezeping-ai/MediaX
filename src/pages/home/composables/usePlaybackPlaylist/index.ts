import { computed, ref, type Ref } from "vue";
import { useStorage } from "@vueuse/core";
import type { PlaybackAdvanceMode, PlaylistItem, UrlPlaylistItem } from "./types";
import {
  buildPlaylistTitle,
  createPlaylistItem,
  pickShuffleNext,
  reorderListItems,
  resolveSequentialNext,
  resolveSequentialPrevious,
  sanitizeHistoryItems,
  sanitizeQueueItems,
  sortHistoryItems,
} from "./playlistHelpers";
import { mergeLegacyUrlPlaylist } from "./migrateLegacyUrlPlaylist";

const QUEUE_STORAGE_KEY = "mediax:playback-queue";
const HISTORY_STORAGE_KEY = "mediax:playback-history";
const ADVANCE_MODE_STORAGE_KEY = "mediax:playback-advance-mode";
const MAX_QUEUE_SIZE = 200;
const MAX_HISTORY_SIZE = 100;

const VALID_ADVANCE_MODES = new Set<PlaybackAdvanceMode>([
  "sequential",
  "shuffle",
  "repeat-one",
  "stop-after-current",
]);

type UsePlaybackPlaylistOptions = {
  currentSource: Ref<string>;
  metadataTitle: Ref<string>;
  openSource: (source: string) => Promise<void>;
  stopPlayback?: () => Promise<void>;
};

export function usePlaybackPlaylist(options: UsePlaybackPlaylistOptions) {
  const queue = useStorage<PlaylistItem[]>(QUEUE_STORAGE_KEY, [], localStorage);
  const history = useStorage<PlaylistItem[]>(HISTORY_STORAGE_KEY, [], localStorage);
  const advanceMode = useStorage<PlaybackAdvanceMode>(ADVANCE_MODE_STORAGE_KEY, "sequential", localStorage);
  const currentPlayingId = ref("");
  const panelVisible = ref(false);
  let handlingTrackEnd = false;

  if (!VALID_ADVANCE_MODES.has(advanceMode.value)) {
    advanceMode.value = "sequential";
  }

  queue.value = sanitizeQueueItems(queue.value, MAX_QUEUE_SIZE);
  history.value = sanitizeHistoryItems(mergeLegacyUrlPlaylist(history.value), MAX_HISTORY_SIZE);

  const sortedHistory = computed(() => sortHistoryItems(history.value));
  const urlPlaylist = computed<UrlPlaylistItem[]>(() =>
    sortedHistory.value
      .filter((item) => item.kind === "url")
      .map((item) => ({
        url: item.source,
        lastOpenedAt: item.lastPlayedAt ?? item.addedAt,
      })),
  );

  const currentIndex = computed(() => {
    const byPlayingId = queue.value.findIndex((item) => item.id === currentPlayingId.value);
    if (byPlayingId >= 0) {
      return byPlayingId;
    }
    const source = options.currentSource.value.trim();
    if (!source) {
      return -1;
    }
    return queue.value.findIndex((item) => item.id === source);
  });
  const hasNext = computed(() => currentIndex.value >= 0 && currentIndex.value < queue.value.length - 1);
  const hasPrevious = computed(() => currentIndex.value > 0);

  function touchHistory(source: string, title?: string) {
    const now = Date.now();
    const existing = history.value.find((item) => item.id === source);
    const nextItem = existing
      ? {
          ...existing,
          title: title?.trim() || existing.title || buildPlaylistTitle(source, options.metadataTitle.value),
          lastPlayedAt: now,
        }
      : createPlaylistItem(source, title ?? options.metadataTitle.value, now);
    history.value = sanitizeHistoryItems(
      [nextItem, ...history.value.filter((item) => item.id !== source)],
      MAX_HISTORY_SIZE,
    );
  }

  function ensureInQueue(source: string, title?: string) {
    if (queue.value.some((item) => item.id === source)) {
      return;
    }
    queue.value = sanitizeQueueItems(
      [...queue.value, createPlaylistItem(source, title ?? options.metadataTitle.value)],
      MAX_QUEUE_SIZE,
    );
  }

  function recordSourceOpened(source: string, title?: string) {
    const trimmed = source.trim();
    if (!trimmed) {
      return;
    }
    touchHistory(trimmed, title);
    ensureInQueue(trimmed, title);
    currentPlayingId.value = trimmed;
  }

  function reorderQueue(oldIndex: number, newIndex: number) {
    const next = reorderListItems(queue.value, oldIndex, newIndex);
    if (next) {
      queue.value = next;
    }
  }

  function removeFromQueue(id: string) {
    queue.value = queue.value.filter((item) => item.id !== id);
    if (currentPlayingId.value === id) {
      currentPlayingId.value = options.currentSource.value.trim();
    }
  }

  function clearQueue() {
    queue.value = [];
    currentPlayingId.value = options.currentSource.value.trim();
  }

  function removeFromHistory(id: string) {
    history.value = history.value.filter((item) => item.id !== id);
    if (currentPlayingId.value === id) {
      currentPlayingId.value = options.currentSource.value.trim();
    }
  }

  function clearHistory() {
    history.value = [];
  }

  function removeUrlFromHistory(url: string) {
    removeFromHistory(url.trim());
  }

  function clearUrlHistory() {
    history.value = history.value.filter((item) => item.kind !== "url");
  }

  function addManyToQueue(sources: string[], titles?: Record<string, string>) {
    const existingIds = new Set(queue.value.map((item) => item.id));
    const nextItems = sources
      .map((source) => source.trim())
      .filter(Boolean)
      .filter((source) => !existingIds.has(source))
      .map((source) => createPlaylistItem(source, titles?.[source]));
    if (!nextItems.length) {
      return 0;
    }
    queue.value = sanitizeQueueItems([...queue.value, ...nextItems], MAX_QUEUE_SIZE);
    return nextItems.length;
  }

  async function importSources(sources: string[], importOptions?: { playFirst?: boolean }) {
    const trimmed = sources.map((source) => source.trim()).filter(Boolean);
    if (!trimmed.length) {
      return 0;
    }
    addManyToQueue(trimmed);
    const shouldPlayFirst =
      importOptions?.playFirst
      ?? (!options.currentSource.value.trim() && !currentPlayingId.value);
    if (shouldPlayFirst) {
      await playSource(trimmed[0]!);
    }
    return trimmed.length;
  }

  function addToQueue(source: string, title?: string) {
    const trimmed = source.trim();
    if (!trimmed || queue.value.some((item) => item.id === trimmed)) {
      return;
    }
    queue.value = sanitizeQueueItems(
      [...queue.value, createPlaylistItem(trimmed, title)],
      MAX_QUEUE_SIZE,
    );
  }

  async function playSource(source: string, title?: string) {
    const trimmed = source.trim();
    if (!trimmed) {
      return;
    }
    await options.openSource(trimmed);
    recordSourceOpened(trimmed, title);
  }

  async function playQueueItem(id: string) {
    const item = queue.value.find((entry) => entry.id === id);
    if (!item) {
      return;
    }
    await playSource(item.source, item.title);
  }

  async function playHistoryItem(id: string) {
    const item = history.value.find((entry) => entry.id === id);
    if (!item) {
      return;
    }
    await playSource(item.source, item.title);
  }

  async function tryPlayNextInQueue() {
    const nextItem = resolveSequentialNext(queue.value, currentIndex.value);
    if (!nextItem) {
      return false;
    }
    await playSource(nextItem.source, nextItem.title);
    return true;
  }

  async function tryPlayPreviousInQueue() {
    const previousItem = resolveSequentialPrevious(queue.value, currentIndex.value);
    if (!previousItem) {
      return false;
    }
    await playSource(previousItem.source, previousItem.title);
    return true;
  }

  function resolveCurrentItem() {
    const id = currentPlayingId.value || options.currentSource.value.trim();
    if (!id) {
      return null;
    }
    return queue.value.find((item) => item.id === id) ?? null;
  }

  async function handleTrackEnded() {
    if (handlingTrackEnd) {
      return;
    }
    handlingTrackEnd = true;
    try {
      switch (advanceMode.value) {
        case "repeat-one": {
          const current = resolveCurrentItem();
          const source = current?.source ?? (currentPlayingId.value || options.currentSource.value.trim());
          if (source) {
            await playSource(source, current?.title);
            return;
          }
          break;
        }
        case "stop-after-current":
          await options.stopPlayback?.();
          return;
        case "shuffle": {
          const nextItem = pickShuffleNext(queue.value, currentPlayingId.value);
          if (nextItem) {
            await playSource(nextItem.source, nextItem.title);
            return;
          }
          break;
        }
        case "sequential":
        default: {
          const nextItem = resolveSequentialNext(queue.value, currentIndex.value);
          if (nextItem) {
            await playSource(nextItem.source, nextItem.title);
            return;
          }
        }
      }
      await options.stopPlayback?.();
    } finally {
      handlingTrackEnd = false;
    }
  }

  function setAdvanceMode(mode: PlaybackAdvanceMode) {
    if (VALID_ADVANCE_MODES.has(mode)) {
      advanceMode.value = mode;
    }
  }

  function togglePanel(force?: boolean) {
    panelVisible.value = force ?? !panelVisible.value;
  }

  return {
    queue: computed(() => queue.value),
    history: sortedHistory,
    urlPlaylist,
    advanceMode: computed(() => advanceMode.value),
    currentPlayingId: computed(() => currentPlayingId.value),
    panelVisible,
    queueCount: computed(() => queue.value.length),
    hasNext,
    hasPrevious,
    addManyToQueue,
    addToQueue,
    clearHistory,
    clearQueue,
    clearUrlHistory,
    importSources,
    playHistoryItem,
    playQueueItem,
    recordSourceOpened,
    removeFromHistory,
    removeFromQueue,
    removeUrlFromHistory,
    reorderQueue,
    togglePanel,
    setAdvanceMode,
    handleTrackEnded,
    tryPlayNextInQueue,
    tryPlayPreviousInQueue,
  };
}

export type PlaybackPlaylistController = ReturnType<typeof usePlaybackPlaylist>;
