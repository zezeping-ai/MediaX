import { computed, ref, watch } from "vue";
import type { PlaybackQualityMode } from "@/modules/media-types";
import { QUALITY_DOWNGRADE_LEVELS } from "../../../components/PlaybackControls/playbackControls.constants";
import { buildPlaybackQualityOptions } from "../../../components/PlaybackControls/playbackControlsUtils";
import { createSourceHeightBaselineCache } from "./createSourceHeightBaselineCache";
import {
  QUALITY_BASELINE_STORAGE_KEY,
  type UsePlaybackQualityControllerOptions,
} from "./types";

export function usePlaybackQualityController(options: UsePlaybackQualityControllerOptions) {
  const { readCachedSourceHeightBaseline, writeCachedSourceHeightBaseline } =
    createSourceHeightBaselineCache(QUALITY_BASELINE_STORAGE_KEY);
  const selectedQuality = ref("source");
  const sourceVideoHeightBaseline = ref<number | null>(null);

  const adaptiveQualitySupported = computed(() =>
    Boolean(options.playback.value?.adaptive_quality_supported),
  );
  const qualitySwitchEnabled = computed(() => {
    const playback = options.playback.value;
    if (!playback || playback.media_kind !== "video") {
      return false;
    }
    const hasVideoDimensions =
      typeof options.metadataVideoHeight.value === "number"
      && Number.isFinite(options.metadataVideoHeight.value)
      && options.metadataVideoHeight.value > 0;
    // Wait until video traits are stable to avoid quality selector flicker
    // when opening audio sources whose initial snapshot still reports `video`.
    return hasVideoDimensions || adaptiveQualitySupported.value;
  });
  const playbackQualityOptions = computed(() => {
    if (!qualitySwitchEnabled.value) {
      return [{ key: "source", label: "原画" }];
    }
    return buildPlaybackQualityOptions(
      sourceVideoHeightBaseline.value,
      QUALITY_DOWNGRADE_LEVELS,
      adaptiveQualitySupported.value,
      selectedQuality.value,
    );
  });

  async function changeQuality(nextQuality: string) {
    if (!qualitySwitchEnabled.value) {
      selectedQuality.value = "source";
      return;
    }
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
    qualitySwitchEnabled,
    selectedQuality,
    sourceVideoHeightBaseline,
  };
}
