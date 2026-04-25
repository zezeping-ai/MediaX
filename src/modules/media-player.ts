import {
  invokeMediaCommand,
  invokeMediaCommandValidated,
  invokeMediaCommandWithRequestIdValidated,
} from "./media-command";
import {
  isMediaSnapshot,
  isPreviewFrame,
  type HardwareDecodeMode,
  type MediaSnapshot,
  type PlaybackQualityMode,
  type PreviewFrame,
} from "./media-types";
import type { PlayerVideoScaleMode } from "./preferences";

export const DEFAULT_PREVIEW_FRAME_MAX_WIDTH = 160;
export const DEFAULT_PREVIEW_FRAME_MAX_HEIGHT = 90;
const MIN_PLAYBACK_RATE = 0.25;
const MAX_PLAYBACK_RATE = 4;
const MIN_PREVIEW_EDGE = 32;
const MAX_PREVIEW_EDGE = 4096;

export interface SeekMediaOptions {
  forceRender?: boolean;
  requestId?: string;
}

function normalizeNonNegative(value: number, field: string) {
  if (!Number.isFinite(value)) {
    throw new Error(`${field} must be a finite number`);
  }
  return Math.max(value, 0);
}

function normalizePlaybackRate(playbackRate: number) {
  if (!Number.isFinite(playbackRate)) {
    throw new Error("playbackRate must be a finite number");
  }
  return Math.min(MAX_PLAYBACK_RATE, Math.max(MIN_PLAYBACK_RATE, playbackRate));
}

function normalizeUnitInterval(value: number, field: string) {
  if (!Number.isFinite(value)) {
    throw new Error(`${field} must be a finite number`);
  }
  return Math.min(1, Math.max(0, value));
}

function normalizePreviewEdge(value: number) {
  if (!Number.isFinite(value)) {
    throw new Error("preview edge must be a finite number");
  }
  return Math.min(MAX_PREVIEW_EDGE, Math.max(MIN_PREVIEW_EDGE, Math.round(value)));
}

export function getPlaybackSnapshot() {
  return invokeMediaCommandValidated<MediaSnapshot>("playback_get_snapshot", isMediaSnapshot);
}

export function playbackOpenSource(path: string) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_open_source", isMediaSnapshot, { path });
}

export function playbackResume() {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_resume", isMediaSnapshot);
}

export function playbackPause() {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_pause", isMediaSnapshot);
}

export function playbackStopSession() {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_stop_session", isMediaSnapshot);
}

export function playbackSeekTo(positionSeconds: number, options: SeekMediaOptions = {}) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_seek_to",
    isMediaSnapshot,
    {
      positionSeconds: normalizeNonNegative(positionSeconds, "positionSeconds"),
      forceRender: options.forceRender ?? false,
    },
    options.requestId,
  );
}

export function playbackSetRate(playbackRate: number) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_set_rate", isMediaSnapshot, {
    playbackRate: normalizePlaybackRate(playbackRate),
  });
}

export function playbackSetVolume(volume: number) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_set_volume", isMediaSnapshot, {
    volume: normalizeUnitInterval(volume, "volume"),
  });
}

export function playbackSetMuted(muted: boolean) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_set_muted", isMediaSnapshot, { muted });
}

export function playbackConfigureDecoderMode(mode: HardwareDecodeMode) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_configure_decoder_mode", isMediaSnapshot, {
    mode,
  });
}

export function playbackSyncPosition(positionSeconds: number, durationSeconds: number) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_sync_position",
    isMediaSnapshot,
    {
      positionSeconds: normalizeNonNegative(positionSeconds, "positionSeconds"),
      durationSeconds: normalizeNonNegative(durationSeconds, "durationSeconds"),
    },
  );
}

export function playbackSetQuality(mode: PlaybackQualityMode) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>("playback_set_quality", isMediaSnapshot, { mode });
}

export function playbackPreviewFrame(
  positionSeconds: number,
  maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
) {
  return invokeMediaCommandValidated<PreviewFrame | null>(
    "playback_preview_frame",
    (value): value is PreviewFrame | null => value === null || isPreviewFrame(value),
    {
    positionSeconds: normalizeNonNegative(positionSeconds, "positionSeconds"),
    maxWidth: normalizePreviewEdge(maxWidth),
    maxHeight: normalizePreviewEdge(maxHeight),
    },
  );
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
export const setMediaQuality = playbackSetQuality;
export const previewMediaFrame = playbackPreviewFrame;

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
}
