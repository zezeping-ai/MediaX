import type { ComputedRef, Ref } from "vue";
import type { PlaybackQualityMode, PlaybackState } from "@/modules/media-types";

export const QUALITY_BASELINE_STORAGE_KEY = "mediax:quality-baseline-by-source";

export interface UsePlaybackQualityControllerOptions {
  currentSource: Ref<string>;
  playback: ComputedRef<PlaybackState | null>;
  metadataVideoHeight: Ref<number | null>;
  setQuality: (mode: PlaybackQualityMode) => Promise<void>;
}
