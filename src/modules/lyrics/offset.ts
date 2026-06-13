import type { MediaLyricLine } from "@/modules/media-types";

/** 歌词同步偏移：syncedTime = playbackTimeSeconds - offsetSeconds */
export function computeLyricsOffsetForLine(
  lines: MediaLyricLine[],
  lineIndex: number,
  playbackTimeSeconds: number,
): number {
  const line = lines[lineIndex];
  if (!line || !Number.isFinite(playbackTimeSeconds)) {
    return 0;
  }
  return playbackTimeSeconds - line.time_seconds;
}

export function resolveActiveLyricIndex(
  lines: MediaLyricLine[],
  playbackTimeSeconds: number,
  offsetSeconds: number,
): number {
  if (lines.length === 0) {
    return -1;
  }
  if (!Number.isFinite(playbackTimeSeconds)) {
    return 0;
  }
  const syncedTime = playbackTimeSeconds - offsetSeconds;
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    if (syncedTime >= lines[index].time_seconds) {
      return index;
    }
  }
  return 0;
}

export function orderLyricLines(lines: MediaLyricLine[]) {
  return lines
    .filter((line) => Number.isFinite(line.time_seconds) && line.text.trim())
    .sort((a, b) => a.time_seconds - b.time_seconds);
}

export function hasSyncedLyricTimings(lines: MediaLyricLine[]) {
  if (lines.length < 2) {
    return false;
  }
  return lines.some((line, index) => index > 0 && line.time_seconds !== lines[index - 1].time_seconds);
}

export function formatLyricsOffsetLabel(offsetSeconds: number) {
  const rounded = Math.round(offsetSeconds * 10) / 10;
  if (Math.abs(rounded) < 0.05) {
    return "0s";
  }
  return rounded > 0 ? `+${rounded}s` : `${rounded}s`;
}
