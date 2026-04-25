import {
  invokeMediaCommand,
  invokeMediaCommandValidated,
  invokeMediaCommandWithRequestIdValidated,
} from "./media-command";
import {
  isMediaSnapshot,
  isPreviewFrame,
  type CacheRecordingStatus,
  type HardwareDecodeMode,
  type MediaSnapshot,
  type PlaybackQualityMode,
  type PreviewFrame,
} from "./media-types";
import type { PlayerVideoScaleMode } from "./preferences";
import {
  normalizeNonNegative,
  normalizePlaybackRate,
  normalizePreviewEdge,
  normalizeUnitInterval,
} from "./player-constraints";

export const DEFAULT_PREVIEW_FRAME_MAX_WIDTH = 160;
export const DEFAULT_PREVIEW_FRAME_MAX_HEIGHT = 90;

export interface SeekMediaOptions {
  forceRender?: boolean;
  requestId?: string;
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

export function playbackGetCacheRecordingStatus() {
  return invokeMediaCommand<CacheRecordingStatus>("playback_get_cache_recording_status");
}

export function playbackStartCacheRecording(outputDir?: string) {
  return invokeMediaCommand<CacheRecordingStatus>("playback_start_cache_recording", {
    outputDir: outputDir ?? null,
  });
}

export function playbackStopCacheRecording() {
  return invokeMediaCommand<CacheRecordingStatus>("playback_stop_cache_recording");
}

export function setMainWindowAlwaysOnTop(enabled: boolean) {
  return invokeMediaCommand<void>("window_set_main_always_on_top", { enabled });
}

export function setMainWindowVideoScaleMode(mode: PlayerVideoScaleMode) {
  return invokeMediaCommand<void>("window_set_main_video_scale_mode", { mode });
}
