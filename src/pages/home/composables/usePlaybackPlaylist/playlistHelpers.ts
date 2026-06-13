import { orderBy, sample, uniqBy } from "lodash-es";
import type { PlaylistItem, PlaylistItemKind } from "./types";

const URL_PROTOCOL_PATTERN = /^(https?|rtsp|rtmp|mms):/i;

export function inferSourceKind(source: string): PlaylistItemKind {
  return URL_PROTOCOL_PATTERN.test(source.trim()) ? "url" : "local";
}

export function buildPlaylistTitle(source: string, metadataTitle?: string | null) {
  const normalizedTitle = metadataTitle?.trim();
  if (normalizedTitle) {
    return normalizedTitle;
  }
  const trimmed = source.trim();
  if (inferSourceKind(trimmed) === "url") {
    try {
      const parsed = new URL(trimmed);
      const tail = decodeURIComponent(parsed.pathname.split("/").filter(Boolean).pop() ?? "");
      return tail || parsed.hostname || trimmed;
    } catch {
      return trimmed;
    }
  }
  return trimmed.split(/[/\\]/).pop() || trimmed;
}

export function createPlaylistItem(source: string, title?: string, lastPlayedAt?: number | null): PlaylistItem {
  const trimmed = source.trim();
  const now = Date.now();
  return {
    id: trimmed,
    source: trimmed,
    kind: inferSourceKind(trimmed),
    title: title?.trim() || buildPlaylistTitle(trimmed),
    addedAt: now,
    lastPlayedAt: lastPlayedAt ?? null,
  };
}

export function sortHistoryItems(items: PlaylistItem[]) {
  return orderBy(
    items,
    [(item) => item.lastPlayedAt ?? item.addedAt],
    ["desc"],
  );
}

export function reorderListItems<T>(items: T[], oldIndex: number, newIndex: number): T[] | null {
  if (oldIndex < 0 || newIndex < 0 || oldIndex >= items.length || newIndex >= items.length) {
    return null;
  }
  const next = items.slice();
  const [moved] = next.splice(oldIndex, 1);
  if (!moved) {
    return null;
  }
  next.splice(newIndex, 0, moved);
  return next;
}

export function normalizePlaylistItem(item: PlaylistItem): PlaylistItem | null {
  const source = item.source?.trim();
  if (!source) {
    return null;
  }
  const normalized = createPlaylistItem(source, item.title, item.lastPlayedAt);
  normalized.addedAt = Number.isFinite(item.addedAt) ? item.addedAt : normalized.addedAt;
  return normalized;
}

export function sanitizeHistoryItems(items: PlaylistItem[], maxSize: number) {
  const deduped = new Map<string, PlaylistItem>();
  for (const item of items) {
    const normalized = normalizePlaylistItem(item);
    if (!normalized) {
      continue;
    }
    const prev = deduped.get(normalized.id);
    if (!prev || (normalized.lastPlayedAt ?? 0) >= (prev.lastPlayedAt ?? 0)) {
      deduped.set(normalized.id, normalized);
    }
  }
  return sortHistoryItems(Array.from(deduped.values())).slice(0, maxSize);
}

export function resolveSequentialNext(
  queue: PlaylistItem[],
  currentIndex: number,
): PlaylistItem | null {
  if (currentIndex < 0 || currentIndex >= queue.length - 1) {
    return null;
  }
  return queue[currentIndex + 1] ?? null;
}

export function resolveSequentialPrevious(
  queue: PlaylistItem[],
  currentIndex: number,
): PlaylistItem | null {
  if (currentIndex <= 0) {
    return null;
  }
  return queue[currentIndex - 1] ?? null;
}

export function pickShuffleNext(queue: PlaylistItem[], currentId: string): PlaylistItem | null {
  if (!queue.length) {
    return null;
  }
  const candidates = queue.length === 1
    ? queue
    : queue.filter((item) => item.id !== currentId);
  return sample(candidates.length ? candidates : queue) ?? null;
}

export function sanitizeQueueItems(items: PlaylistItem[], maxSize: number) {
  return uniqBy(
    items
      .map(normalizePlaylistItem)
      .filter((item): item is PlaylistItem => item !== null),
    "id",
  ).slice(0, maxSize);
}
