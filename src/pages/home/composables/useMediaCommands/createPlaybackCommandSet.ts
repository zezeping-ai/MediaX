import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  playbackConfigureDecoderMode,
  playbackOpenSource,
  playbackPause,
  playbackPreviewFrame,
  playbackResume,
  playbackSeekTo,
  playbackSetChannelRouting,
  playbackSetLeftChannelMuted,
  playbackSetLeftChannelVolume,
  playbackSetMuted,
  playbackSetQuality,
  playbackSetRate,
  playbackSetRightChannelMuted,
  playbackSetRightChannelVolume,
  playbackSetVolume,
  playbackStopSession,
  playbackSyncPosition,
} from "@/modules/media-player";
import type {
  HardwareDecodeMode,
  MediaSnapshot,
  PlaybackChannelRouting,
  PlaybackQualityMode,
  PreviewFrame,
} from "@/modules/media-types";

export interface PlaybackCommandSet {
  openSource: (source: string) => Promise<MediaSnapshot>;
  play: () => Promise<MediaSnapshot>;
  pause: () => Promise<MediaSnapshot>;
  stop: () => Promise<MediaSnapshot>;
  seek: (positionSeconds: number, forceRender?: boolean) => Promise<MediaSnapshot>;
  setRate: (playbackRate: number) => Promise<MediaSnapshot>;
  setVolume: (volume: number) => Promise<MediaSnapshot>;
  setMuted: (muted: boolean) => Promise<MediaSnapshot>;
  setLeftChannelVolume: (volume: number) => Promise<MediaSnapshot>;
  setRightChannelVolume: (volume: number) => Promise<MediaSnapshot>;
  setLeftChannelMuted: (muted: boolean) => Promise<MediaSnapshot>;
  setRightChannelMuted: (muted: boolean) => Promise<MediaSnapshot>;
  setChannelRouting: (routing: PlaybackChannelRouting) => Promise<MediaSnapshot>;
  setQuality: (mode: PlaybackQualityMode) => Promise<MediaSnapshot>;
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  syncPosition: (positionSeconds: number, durationSeconds: number) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
}

export function createPlaybackCommandSet(): PlaybackCommandSet {
  return {
    openSource: (source) => playbackOpenSource(source),
    play: () => playbackResume(),
    pause: () => playbackPause(),
    stop: () => playbackStopSession(),
    seek: (positionSeconds, forceRender = false) => playbackSeekTo(positionSeconds, { forceRender }),
    setRate: (playbackRate) => playbackSetRate(playbackRate),
    setVolume: (volume) => playbackSetVolume(volume),
    setMuted: (muted) => playbackSetMuted(muted),
    setLeftChannelVolume: (volume) => playbackSetLeftChannelVolume(volume),
    setRightChannelVolume: (volume) => playbackSetRightChannelVolume(volume),
    setLeftChannelMuted: (muted) => playbackSetLeftChannelMuted(muted),
    setRightChannelMuted: (muted) => playbackSetRightChannelMuted(muted),
    setChannelRouting: (routing) => playbackSetChannelRouting(routing),
    setQuality: (mode) => playbackSetQuality(mode),
    configureDecoderMode: (mode) => playbackConfigureDecoderMode(mode),
    syncPosition: (positionSeconds, durationSeconds) =>
      playbackSyncPosition(positionSeconds, durationSeconds),
    requestPreviewFrame: (
      positionSeconds,
      maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
      maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
    ) => playbackPreviewFrame(positionSeconds, maxWidth, maxHeight),
  };
}
