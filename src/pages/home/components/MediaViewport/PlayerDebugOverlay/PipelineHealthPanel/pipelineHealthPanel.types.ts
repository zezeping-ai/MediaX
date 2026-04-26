import type { MediaTelemetryPayload } from "@/modules/media-types";

export interface HealthLane {
  id: string;
  label: string;
  percent: number;
  state: string;
  detail: string;
  toneClass: string;
  points: string;
  trendLabel: string;
  trendDelta: string;
  markerPercents: number[];
}

export interface PipelineHealthPanelProps {
  telemetry: MediaTelemetryPayload | null;
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
}
