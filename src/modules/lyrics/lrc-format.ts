import type { MediaLyricLine } from "@/modules/media-types";
import { hasSyncedLyricTimings, orderLyricLines } from "./offset";

function formatLrcTimestamp(seconds: number) {
  const safeSeconds = Math.max(0, seconds);
  const minutes = Math.floor(safeSeconds / 60);
  const remainder = safeSeconds - minutes * 60;
  const mm = String(minutes).padStart(2, "0");
  const ss = remainder.toFixed(2).padStart(5, "0");
  return `${mm}:${ss}`;
}

export function formatLyricsToLrc(lines: MediaLyricLine[]) {
  const ordered = orderLyricLines(lines);
  if (ordered.length === 0) {
    return "";
  }
  if (!hasSyncedLyricTimings(ordered)) {
    return ordered.map((line) => line.text).join("\n");
  }
  return ordered
    .map((line) => `[${formatLrcTimestamp(line.time_seconds)}]${line.text}`)
    .join("\n");
}
