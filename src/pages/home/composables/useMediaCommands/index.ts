import type { CacheRecordingStatus, HardwareDecodeMode, MediaSnapshot, PlaybackQualityMode, PreviewFrame } from "@/modules/media-types";
import {
  createPlaybackCommandSet,
  type PlaybackCommandSet,
} from "./createPlaybackCommandSet";
import {
  createCacheRecordingCommandSet,
  type CacheRecordingCommandSet,
} from "./createCacheRecordingCommandSet";

export interface MediaCommandSet extends PlaybackCommandSet, CacheRecordingCommandSet {
  openPath: (path: string) => Promise<MediaSnapshot>;
  play: () => Promise<MediaSnapshot>;
  pause: () => Promise<MediaSnapshot>;
  stop: () => Promise<MediaSnapshot>;
  seek: (positionSeconds: number, forceRender?: boolean) => Promise<MediaSnapshot>;
  setRate: (playbackRate: number) => Promise<MediaSnapshot>;
  setVolume: (volume: number) => Promise<MediaSnapshot>;
  setMuted: (muted: boolean) => Promise<MediaSnapshot>;
  setQuality: (mode: PlaybackQualityMode) => Promise<MediaSnapshot>;
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  syncPosition: (positionSeconds: number, durationSeconds: number) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
  getCacheRecordingStatus: () => Promise<CacheRecordingStatus>;
  startCacheRecording: (outputDir?: string) => Promise<CacheRecordingStatus>;
  stopCacheRecording: () => Promise<CacheRecordingStatus>;
}

export function useMediaCommands(): MediaCommandSet {
  const playbackCommands = createPlaybackCommandSet();
  const cacheRecordingCommands = createCacheRecordingCommandSet();

  return {
    ...playbackCommands,
    ...cacheRecordingCommands,
  };
}
