import { computed } from "vue";
import { buildPipelineHealthLanes } from "./pipelineHealthPanel.utils";
import type { PipelineHealthPanelProps } from "./pipelineHealthPanel.types";

export function usePipelineHealthLanes(props: PipelineHealthPanelProps) {
  const lanes = computed(() =>
    buildPipelineHealthLanes(props.source, props.playback, props.telemetry, props.history),
  );

  return {
    lanes,
  };
}
