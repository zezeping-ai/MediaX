import type { LyricsCandidateSummary, LyricsSearchHit } from "@/modules/media-types";
import type { LyricsSelectOption } from "./types";

function extractArtistTitleFromLabel(label: string) {
  const body = label.split("·").slice(1).join("·").trim() || label.trim();
  const split = body.split(" - ");
  if (split.length >= 2) {
    return {
      artist: split[0]?.trim() || "",
      title: split.slice(1).join(" - ").trim(),
    };
  }
  return { artist: "", title: body };
}

export function fromSearchHit(hit: LyricsSearchHit): LyricsSelectOption {
  return {
    id: hit.id,
    provider_id: hit.provider_id,
    title: hit.title.trim() || hit.preview.trim() || "未知歌曲",
    artist: hit.artist.trim() || "未知演唱者",
    album: hit.album,
    duration_seconds: hit.duration_seconds,
    preview: hit.preview,
    synced: hit.synced,
    lyrics_lrc: hit.lyrics_lrc,
  };
}

export function fromLocalLyrics(input: {
  lyricsLrc: string;
  source: string | null;
  title: string;
  artist: string;
  album?: string;
  durationSeconds?: number;
}): LyricsSelectOption | null {
  const lyricsLrc = input.lyricsLrc.trim();
  if (!lyricsLrc) {
    return null;
  }
  const providerId = input.source === "sidecar" ? "sidecar" : "embedded";
  const previewLine = lyricsLrc
    .split("\n")
    .map((line) => line.trim())
    .find((line) => line.length > 0)
    ?.replace(/^\[[^\]]+\]/, "")
    .trim();
  return {
    id: `local:${providerId}`,
    provider_id: providerId,
    title: input.title.trim() || "未知歌曲",
    artist: input.artist.trim() || "未知演唱者",
    album: input.album,
    duration_seconds: input.durationSeconds,
    preview: previewLine || lyricsLrc.slice(0, 48),
    synced: /\[\d{1,2}:\d{2}/.test(lyricsLrc),
    lyrics_lrc: lyricsLrc,
  };
}

export function isLocalLyricsOption(option: LyricsSelectOption) {
  return option.provider_id === "embedded" || option.provider_id === "sidecar";
}

export function fromCandidate(candidate: LyricsCandidateSummary): LyricsSelectOption {
  const parsed = extractArtistTitleFromLabel(candidate.label);
  return {
    id: candidate.id,
    provider_id: candidate.provider_id,
    title: candidate.track_name?.trim() || parsed.title || candidate.preview.trim() || candidate.label,
    artist: candidate.artist_name?.trim() || parsed.artist || "未知演唱者",
    album: null,
    duration_seconds: candidate.duration_seconds,
    preview: candidate.preview,
    synced: candidate.synced,
  };
}
