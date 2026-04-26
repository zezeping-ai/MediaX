import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import { useLocalStorage } from "@vueuse/core";
import type { PlaybackQualityMode, PlaybackState } from "@/modules/media-types";
import { QUALITY_DOWNGRADE_LEVELS } from "../../components/PlaybackControls/playbackControls.constants";
import { buildPlaybackQualityOptions } from "../../components/PlaybackControls/playbackControlsUtils";

const QUALITY_BASELINE_STORAGE_KEY = "mediax:quality-baseline-by-source";

type UsePlaybackQualityControllerOptions = {
  currentSource: Ref<string>;
  playback: ComputedRef<PlaybackState | null>;
  metadataVideoHeight: Ref<number | null>;
  setQuality: (mode: PlaybackQualityMode) => Promise<void>;
};

export function usePlaybackQualityController(options: UsePlaybackQualityControllerOptions) {
  const sourceHeightBaselineByPath = useLocalStorage<Record<string, number>>(
    QUALITY_BASELINE_STORAGE_KEY,
    {},
  );
  const selectedQuality = ref("source");
  const sourceVideoHeightBaseline = ref<number | null>(null);

  const adaptiveQualitySupported = computed(() =>
    Boolean(options.playback.value?.adaptive_quality_supported),
  );
  const playbackQualityOptions = computed(() =>
    buildPlaybackQualityOptions(
      sourceVideoHeightBaseline.value,
      QUALITY_DOWNGRADE_LEVELS,
      adaptiveQualitySupported.value,
      selectedQuality.value,
    ),
  );

  async function changeQuality(nextQuality: string) {
    const nextMode = nextQuality as PlaybackQualityMode;
    selectedQuality.value = nextQuality;
    await options.setQuality(nextMode);
  }

  watch(options.currentSource, () => {
    selectedQuality.value = "source";
    const path = options.currentSource.value;
    sourceVideoHeightBaseline.value = path ? readCachedSourceHeightBaseline(path) : null;
  });

  watch(options.playback, (value) => {
    if (!value) {
      selectedQuality.value = "source";
      return;
    }
    selectedQuality.value = value.quality_mode ?? "source";
  });

  watch(options.metadataVideoHeight, (nextHeight) => {
    if (typeof nextHeight !== "number" || !Number.isFinite(nextHeight) || nextHeight <= 0) {
      return;
    }
    if (sourceVideoHeightBaseline.value === null) {
      sourceVideoHeightBaseline.value = nextHeight;
    } else {
      sourceVideoHeightBaseline.value = Math.max(sourceVideoHeightBaseline.value, nextHeight);
    }
    if (options.currentSource.value && sourceVideoHeightBaseline.value !== null) {
      writeCachedSourceHeightBaseline(
        options.currentSource.value,
        sourceVideoHeightBaseline.value,
      );
    }
  });

  return {
    adaptiveQualitySupported,
    changeQuality,
    playbackQualityOptions,
    selectedQuality,
    sourceVideoHeightBaseline,
  };

  function readCachedSourceHeightBaseline(path: string): number | null {
    if (!path) {
      return null;
    }
    const value = sourceHeightBaselineByPath.value[path];
    return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
  }

  function writeCachedSourceHeightBaseline(path: string, height: number) {
    if (!path || !Number.isFinite(height) || height <= 0) {
      return;
    }
    const nextHeight = Math.round(height);
    const prevHeight = sourceHeightBaselineByPath.value[path];
    sourceHeightBaselineByPath.value[path] =
      typeof prevHeight === "number" && Number.isFinite(prevHeight) && prevHeight > 0
        ? Math.max(prevHeight, nextHeight)
        : nextHeight;
  }
}
