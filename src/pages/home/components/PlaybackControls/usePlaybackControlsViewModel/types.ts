import type { PlaybackState, PreviewFrame } from "@/modules/media-types";
import type { PlaybackQualityOption } from "../playbackControlsUtils";

export type SliderValue = number | [number, number];

export type RequestPreviewFrame = (
  positionSeconds: number,
  maxWidth?: number,
  maxHeight?: number,
) => Promise<PreviewFrame | null>;

export interface PlaybackControlsProps {
  playback: PlaybackState | null;
  disabled: boolean;
  playbackRate: number;
  volume: number;
  muted: boolean;
  locked: boolean;
  cacheRecording: boolean;
  cacheOutputPath: string;
  durationSecondsOverride: number;
  qualityOptions: PlaybackQualityOption[];
  selectedQuality: string;
  requestPreviewFrame?: RequestPreviewFrame;
}

export interface PlaybackControlsEmit {
  (event: "play"): void;
  (event: "pause", position: number): void;
  (event: "stop"): void;
  (event: "seek", position: number): void;
  (event: "seek-preview", position: number): void;
  (event: "change-rate", value: number): void;
  (event: "change-volume", value: number): void;
  (event: "commit-volume", value: number): void;
  (event: "change-quality", value: string): void;
  (event: "overlay-interaction-change", value: boolean): void;
  (event: "toggle-mute"): void;
  (event: "toggle-cache"): void;
  (event: "toggle-lock"): void;
}
