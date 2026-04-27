import {
  invokeMediaCommandValidated,
  invokeMediaCommandWithRequestIdValidated,
} from "../media-command";
import {
  isMediaSnapshot,
  isPreviewFrame,
  type HardwareDecodeMode,
  type MediaSnapshot,
  type PlaybackChannelRouting,
  type PlaybackQualityMode,
  type PreviewFrame,
} from "../media-types";
import {
  normalizeNonNegative,
  normalizePlaybackRate,
  normalizePreviewEdge,
  normalizeUnitInterval,
} from "../player-constraints";
import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
} from "./constants";

export interface SeekMediaOptions {
  forceRender?: boolean;
  requestId?: string;
}

export function getPlaybackSnapshot() {
  return invokeMediaCommandValidated<MediaSnapshot>("playback_get_snapshot", isMediaSnapshot);
}

export function playbackOpenSource(path: string) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_open_source",
    isMediaSnapshot,
    { path },
  );
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

export function playbackSetLeftChannelVolume(volume: number) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_set_left_channel_volume",
    isMediaSnapshot,
    { volume: normalizeUnitInterval(volume, "volume") },
  );
}

export function playbackSetRightChannelVolume(volume: number) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_set_right_channel_volume",
    isMediaSnapshot,
    { volume: normalizeUnitInterval(volume, "volume") },
  );
}

export function playbackSetLeftChannelMuted(muted: boolean) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_set_left_channel_muted",
    isMediaSnapshot,
    { muted },
  );
}

export function playbackSetRightChannelMuted(muted: boolean) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_set_right_channel_muted",
    isMediaSnapshot,
    { muted },
  );
}

export function playbackSetChannelRouting(routing: PlaybackChannelRouting) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_set_channel_routing",
    isMediaSnapshot,
    { routing },
  );
}

export function playbackConfigureDecoderMode(mode: HardwareDecodeMode) {
  return invokeMediaCommandWithRequestIdValidated<MediaSnapshot>(
    "playback_configure_decoder_mode",
    isMediaSnapshot,
    { mode },
  );
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
