import { computed, type Ref } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { createCurrentFrameSections } from "./createCurrentFrameSections";
import { createOverviewSections } from "./createOverviewSections";
import { createPipelineSections } from "./createPipelineSections";
import { createStreamSections } from "./createStreamSections";
import { createTimingSections } from "./createTimingSections";

export function createSectionComputeds(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
  latestTelemetry?: Ref<MediaTelemetryPayload | null>,
) {
  const overviewSections = computed(() =>
    createOverviewSections(playback.value, debugSnapshot.value, latestTelemetry?.value ?? null),
  );
  const streamSections = computed(() => createStreamSections(debugSnapshot.value));
  const timingSections = computed(() =>
    createTimingSections(debugSnapshot.value, latestTelemetry?.value ?? null),
  );
  const pipelineSections = computed(() =>
    createPipelineSections(debugSnapshot.value, latestTelemetry?.value ?? null),
  );
  const currentFrameSections = computed(() =>
    createCurrentFrameSections(
      playback.value,
      debugSnapshot.value,
      latestTelemetry?.value ?? null,
    ),
  );

  return {
    overviewSections,
    streamSections,
    timingSections,
    pipelineSections,
    currentFrameSections,
  };
}
