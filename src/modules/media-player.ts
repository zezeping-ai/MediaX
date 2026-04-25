import { invoke } from "@tauri-apps/api/core";
import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "./media-types";
import type { PlayerVideoScaleMode } from "./preferences";

export const DEFAULT_PREVIEW_FRAME_MAX_WIDTH = 160;
export const DEFAULT_PREVIEW_FRAME_MAX_HEIGHT = 90;

export interface SeekMediaOptions {
  forceRender?: boolean;
  requestId?: string;
}

function nextRequestId(provided?: string) {
  return provided ?? crypto.randomUUID();
}

export function getPlaybackSnapshot() {
  return invoke<MediaSnapshot>("playback_get_snapshot");
}

export function playbackOpenSource(path: string) {
  return invoke<MediaSnapshot>("playback_open_source", { path, requestId: nextRequestId() });
}

export function playbackResume() {
  return invoke<MediaSnapshot>("playback_resume", { requestId: nextRequestId() });
}

export function playbackPause() {
  return invoke<MediaSnapshot>("playback_pause", { requestId: nextRequestId() });
}

export function playbackStopSession() {
  return invoke<MediaSnapshot>("playback_stop_session", { requestId: nextRequestId() });
}

export function playbackSeekTo(positionSeconds: number, options: SeekMediaOptions = {}) {
  return invoke<MediaSnapshot>("playback_seek_to", {
    positionSeconds,
    forceRender: options.forceRender ?? false,
    requestId: nextRequestId(options.requestId),
  });
}

export function playbackSetRate(playbackRate: number) {
  return invoke<MediaSnapshot>("playback_set_rate", { playbackRate, requestId: nextRequestId() });
}

export function playbackSetVolume(volume: number) {
  return invoke<MediaSnapshot>("playback_set_volume", { volume, requestId: nextRequestId() });
}

export function playbackSetMuted(muted: boolean) {
  return invoke<MediaSnapshot>("playback_set_muted", { muted, requestId: nextRequestId() });
}

export function playbackConfigureDecoderMode(mode: HardwareDecodeMode) {
  return invoke<MediaSnapshot>("playback_configure_decoder_mode", { mode, requestId: nextRequestId() });
}

export function playbackSyncPosition(positionSeconds: number, durationSeconds: number) {
  return invoke<MediaSnapshot>("playback_sync_position", {
    positionSeconds,
    durationSeconds,
    requestId: nextRequestId(),
  });
}

export function playbackPreviewFrame(
  positionSeconds: number,
  maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
) {
  return invoke<PreviewFrame | null>("playback_preview_frame", {
    positionSeconds,
    maxWidth,
    maxHeight,
  });
}

// Legacy aliases kept to avoid breaking existing imports while migration is ongoing.
export const getMediaSnapshot = getPlaybackSnapshot;
export const openMedia = playbackOpenSource;
export const playMedia = playbackResume;
export const pauseMedia = playbackPause;
export const stopMedia = playbackStopSession;
export const seekMedia = playbackSeekTo;
export const setMediaRate = playbackSetRate;
export const setMediaVolume = playbackSetVolume;
export const setMediaMuted = playbackSetMuted;
export const setMediaHwDecodeMode = playbackConfigureDecoderMode;
export const syncMediaPosition = playbackSyncPosition;
export const previewMediaFrame = playbackPreviewFrame;

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invoke<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invoke<void>("window_set_main_video_scale_mode", { mode });
}
