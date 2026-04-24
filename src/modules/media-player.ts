import { invoke } from "@tauri-apps/api/core";
import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "./media-types";

export interface SeekMediaOptions {
  forceRender?: boolean;
}

export function getMediaSnapshot() {
  return invoke<MediaSnapshot>("media_get_snapshot");
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

export function seekMedia(positionSeconds: number, options: SeekMediaOptions = {}) {
  return invoke<MediaSnapshot>("media_seek", {
    positionSeconds,
    forceRender: options.forceRender ?? false,
  });
}

export function setMediaRate(playbackRate: number) {
  return invoke<MediaSnapshot>("media_set_rate", { playbackRate });
}

export function setMediaVolume(volume: number) {
  return invoke<MediaSnapshot>("media_set_volume", { volume });
}

export function setMediaMuted(muted: boolean) {
  return invoke<MediaSnapshot>("media_set_muted", { muted });
}

export function setMediaHwDecodeMode(mode: HardwareDecodeMode) {
  return invoke<MediaSnapshot>("media_set_hw_decode_mode", { mode });
}

export function syncMediaPosition(positionSeconds: number, durationSeconds: number) {
  return invoke<MediaSnapshot>("media_sync_position", { positionSeconds, durationSeconds });
}

export function previewMediaFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
  return invoke<PreviewFrame | null>("media_preview_frame", {
    positionSeconds,
    maxWidth,
    maxHeight,
  });
}
