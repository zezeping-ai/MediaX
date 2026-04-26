import type { Ref } from "vue";
import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";
import { createDebugSummaryComputeds } from "./createDebugSummaryComputeds";
import { createProcessComputeds } from "./createProcessComputeds";
import { createSectionComputeds } from "./createSectionComputeds";

export type {
  CurrentFrameSection,
  DebugGroup,
  DebugRow,
  DebugSection,
  DecodeBannerState,
  HardwareDecisionEvent,
  ProcessStage,
} from "./types";

export function usePlayerDebugOverlay(
  playback: Ref<PlaybackState | null>,
  debugSnapshot: Ref<Record<string, string>>,
  debugTimeline?: Ref<Array<{ stage: string; message: string; at_ms: number }>>,
  debugStageSnapshot?: Ref<Record<string, { message: string; at_ms: number }>>,
  latestTelemetry?: Ref<MediaTelemetryPayload | null>,
) {
  const { decodeBanner, resourceSummary, debugGroups } = createDebugSummaryComputeds(
    playback,
    debugSnapshot,
  );
  const {
    overviewSections,
    streamSections,
    timingSections,
    pipelineSections,
    currentFrameSections,
  } = createSectionComputeds(playback, debugSnapshot, latestTelemetry);
  const { hardwareDecisionTimeline, processStages } = createProcessComputeds(
    debugTimeline,
    debugStageSnapshot,
  );

  return {
    decodeBanner,
    resourceSummary,
    debugGroups,
    currentFrameSections,
    hardwareDecisionTimeline,
    overviewSections,
    pipelineSections,
    streamSections,
    timingSections,
    processStages,
  };
}
