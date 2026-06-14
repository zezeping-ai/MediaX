import { formatLyricsSourceLabel, formatTrackDuration } from "@/modules/lyrics";
import type { LyricsSelectOption } from "./types";

export function truncateText(value: string, maxChars: number) {
  if (value.length <= maxChars) {
    return value;
  }
  return `${value.slice(0, maxChars - 1)}…`;
}

export function formatLyricsOptionTitle(option: LyricsSelectOption) {
  const provider = formatLyricsSourceLabel(option.provider_id) || option.provider_id;
  return `${provider} · ${option.title}`;
}

export function formatLyricsOptionMeta(option: LyricsSelectOption) {
  const artist = option.artist.trim() || "未知演唱者";
  const duration = formatTrackDuration(option.duration_seconds);
  const album = option.album?.trim();
  return album ? `${artist} · ${duration} · ${album}` : `${artist} · ${duration}`;
}

export function formatLyricsTriggerLabel(option: LyricsSelectOption) {
  const preview = option.preview.trim();
  if (preview) {
    return truncateText(preview, 28);
  }
  return truncateText(option.title, 28);
}

export function buildLyricsOptionFilterLabel(option: LyricsSelectOption) {
  return `${option.title} ${option.artist} ${formatTrackDuration(option.duration_seconds)}`;
}
