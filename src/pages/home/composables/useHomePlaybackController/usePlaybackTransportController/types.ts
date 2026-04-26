import type { ComputedRef } from "vue";
import type { PlaybackState } from "@/modules/media-types";

export interface UsePlaybackTransportControllerOptions {
  playback: ComputedRef<PlaybackState | null>;
  play: () => Promise<void>;
  pause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (positionSeconds: number) => Promise<void>;
  setRate: (rate: number) => Promise<void>;
  setVolume: (volume: number) => Promise<void>;
  setMuted: (muted: boolean) => Promise<void>;
}
