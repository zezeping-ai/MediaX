import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  playbackConfigureDecoderMode,
  playbackOpenSource,
  playbackPause,
  playbackPreviewFrame,
  playbackResume,
  playbackSeekTo,
  playbackSetMuted,
  playbackSetRate,
  playbackSetVolume,
  playbackStopSession,
  playbackSyncPosition,
} from "@/modules/media-player";
import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "@/modules/media-types";

export interface MediaCommandSet {
  openPath: (path: string) => Promise<MediaSnapshot>;
  play: () => Promise<MediaSnapshot>;
  pause: () => Promise<MediaSnapshot>;
  stop: () => Promise<MediaSnapshot>;
  seek: (positionSeconds: number, forceRender?: boolean) => Promise<MediaSnapshot>;
  setRate: (playbackRate: number) => Promise<MediaSnapshot>;
  setVolume: (volume: number) => Promise<MediaSnapshot>;
  setMuted: (muted: boolean) => Promise<MediaSnapshot>;
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  syncPosition: (positionSeconds: number, durationSeconds: number) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
}

export function useMediaCommands(): MediaCommandSet {
  return {
    openPath: (path) => playbackOpenSource(path),
    play: () => playbackResume(),
    pause: () => playbackPause(),
    stop: () => playbackStopSession(),
    seek: (positionSeconds, forceRender = false) => playbackSeekTo(positionSeconds, { forceRender }),
    setRate: (playbackRate) => playbackSetRate(playbackRate),
    setVolume: (volume) => playbackSetVolume(volume),
    setMuted: (muted) => playbackSetMuted(muted),
    configureDecoderMode: (mode) => playbackConfigureDecoderMode(mode),
    syncPosition: (positionSeconds, durationSeconds) =>
      playbackSyncPosition(positionSeconds, durationSeconds),
    requestPreviewFrame: (
      positionSeconds,
      maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
      maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
    ) =>
      playbackPreviewFrame(positionSeconds, maxWidth, maxHeight),
  };
}
