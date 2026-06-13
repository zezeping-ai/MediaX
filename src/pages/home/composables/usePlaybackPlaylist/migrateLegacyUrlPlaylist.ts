import type { PlaylistItem } from "./types";
import { createPlaylistItem, sanitizeHistoryItems } from "./playlistHelpers";

const LEGACY_URL_PLAYLIST_KEY = "mediax:open-url-playlist";
const MIGRATION_FLAG_KEY = "mediax:playback-playlist-migrated-v1";

type LegacyUrlPlaylistItem = {
  url?: string;
  lastOpenedAt?: number;
};

export function mergeLegacyUrlPlaylist(history: PlaylistItem[]) {
  if (localStorage.getItem(MIGRATION_FLAG_KEY)) {
    return history;
  }
  localStorage.setItem(MIGRATION_FLAG_KEY, "1");

  let legacyItems: LegacyUrlPlaylistItem[] = [];
  try {
    legacyItems = JSON.parse(localStorage.getItem(LEGACY_URL_PLAYLIST_KEY) ?? "[]") as LegacyUrlPlaylistItem[];
  } catch {
    legacyItems = [];
  }

  const merged = [...history];
  for (const item of legacyItems) {
    const url = item.url?.trim();
    if (!url) {
      continue;
    }
    merged.push(
      createPlaylistItem(
        url,
        undefined,
        typeof item.lastOpenedAt === "number" && Number.isFinite(item.lastOpenedAt) ? item.lastOpenedAt : Date.now(),
      ),
    );
  }
  return sanitizeHistoryItems(merged, 100);
}
