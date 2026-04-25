import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  openMedia,
  pauseMedia,
  playMedia,
  previewMediaFrame,
  seekMedia,
  setMediaHwDecodeMode,
  setMediaMuted,
  setMediaRate,
  setMediaVolume,
  stopMedia,
  syncMediaPosition,
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
  setHwMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  syncPosition: (positionSeconds: number, durationSeconds: number) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
}

export function useMediaCommands(): MediaCommandSet {
  return {
    openPath: (path) => openMedia(path),
    play: () => playMedia(),
    pause: () => pauseMedia(),
    stop: () => stopMedia(),
    seek: (positionSeconds, forceRender = false) => seekMedia(positionSeconds, { forceRender }),
    setRate: (playbackRate) => setMediaRate(playbackRate),
    setVolume: (volume) => setMediaVolume(volume),
    setMuted: (muted) => setMediaMuted(muted),
    setHwMode: (mode) => setMediaHwDecodeMode(mode),
    syncPosition: (positionSeconds, durationSeconds) =>
      syncMediaPosition(positionSeconds, durationSeconds),
    requestPreviewFrame: (
      positionSeconds,
      maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
      maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
    ) =>
      previewMediaFrame(positionSeconds, maxWidth, maxHeight),
  };
}
