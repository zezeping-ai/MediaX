import { invoke } from "@tauri-apps/api/core";

export type PlaybackStatus = "idle" | "playing" | "paused" | "stopped";

export interface PlaybackState {
  engine: string;
  status: PlaybackStatus;
  current_path: string | null;
  position_seconds: number;
  duration_seconds: number;
  playback_rate: number;
  error: string | null;
}

export interface MediaItem {
  id: string;
  path: string;
  name: string;
  extension: string;
  size_bytes: number;
  last_played_at: number | null;
  last_position_seconds: number;
}

export interface MediaLibraryState {
  roots: string[];
  items: MediaItem[];
}

export interface MediaSnapshot {
  playback: PlaybackState;
  library: MediaLibraryState;
}

export function getMediaSnapshot() {
  return invoke<MediaSnapshot>("media_get_snapshot");
}

export function setMediaLibraryRoots(roots: string[]) {
  return invoke<MediaSnapshot>("media_set_library_roots", { roots });
}

export function rescanMediaLibrary() {
  return invoke<MediaSnapshot>("media_rescan_library");
}

export function openMedia(path: string) {
  return invoke<MediaSnapshot>("media_open", { path });
}

export function playMedia() {
  return invoke<MediaSnapshot>("media_play");
}

export function pauseMedia() {
  return invoke<MediaSnapshot>("media_pause");
}

export function stopMedia() {
  return invoke<MediaSnapshot>("media_stop");
}

export function seekMedia(positionSeconds: number) {
  return invoke<MediaSnapshot>("media_seek", { positionSeconds });
}

export function setMediaRate(playbackRate: number) {
  return invoke<MediaSnapshot>("media_set_rate", { playbackRate });
}

export function syncMediaPosition(positionSeconds: number, durationSeconds: number) {
  return invoke<MediaSnapshot>("media_sync_position", { positionSeconds, durationSeconds });
}
