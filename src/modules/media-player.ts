import { invoke } from "@tauri-apps/api/core";
import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "./media-types";

export interface SeekMediaOptions {
  forceRender?: boolean;
  requestId?: string;
}

function nextRequestId(provided?: string) {
  return provided ?? crypto.randomUUID();
}

export function getMediaSnapshot() {
  return invoke<MediaSnapshot>("media_get_snapshot");
}

export function openMedia(path: string) {
  return invoke<MediaSnapshot>("media_open", { path, requestId: nextRequestId() });
}

export function playMedia() {
  return invoke<MediaSnapshot>("media_play", { requestId: nextRequestId() });
}

export function pauseMedia() {
  return invoke<MediaSnapshot>("media_pause", { requestId: nextRequestId() });
}

export function stopMedia() {
  return invoke<MediaSnapshot>("media_stop", { requestId: nextRequestId() });
}

export function seekMedia(positionSeconds: number, options: SeekMediaOptions = {}) {
  return invoke<MediaSnapshot>("media_seek", {
    positionSeconds,
    forceRender: options.forceRender ?? false,
    requestId: nextRequestId(options.requestId),
  });
}

export function setMediaRate(playbackRate: number) {
  return invoke<MediaSnapshot>("media_set_rate", { playbackRate, requestId: nextRequestId() });
}

export function setMediaVolume(volume: number) {
  return invoke<MediaSnapshot>("media_set_volume", { volume, requestId: nextRequestId() });
}

export function setMediaMuted(muted: boolean) {
  return invoke<MediaSnapshot>("media_set_muted", { muted, requestId: nextRequestId() });
}

export function setMediaHwDecodeMode(mode: HardwareDecodeMode) {
  return invoke<MediaSnapshot>("media_set_hw_decode_mode", { mode, requestId: nextRequestId() });
}

export function syncMediaPosition(positionSeconds: number, durationSeconds: number) {
  return invoke<MediaSnapshot>("media_sync_position", {
    positionSeconds,
    durationSeconds,
    requestId: nextRequestId(),
  });
}

export function previewMediaFrame(positionSeconds: number, maxWidth = 160, maxHeight = 90) {
  return invoke<PreviewFrame | null>("media_preview_frame", {
    positionSeconds,
    maxWidth,
    maxHeight,
  });
}

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invoke<void>("window_set_main_always_on_top", { enabled });
}
