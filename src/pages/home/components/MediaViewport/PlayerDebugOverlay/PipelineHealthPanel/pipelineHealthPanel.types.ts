import type { MediaTelemetryPayload, PlaybackState } from "@/modules/media-types";

export type HealthLaneTone = "unknown" | "headroom" | "healthy" | "tight" | "risk";

export interface HealthLane {
  id: string;
  label: string;
  percent: number;
  state: string;
  detail: string;
  tone: HealthLaneTone;
  trendLabel: string;
  trendDelta: string;
  markerPercents: number[];
  samples: number[];
}

export interface PipelineHealthPanelProps {
  source: string;
  playback: PlaybackState | null;
  telemetry: MediaTelemetryPayload | null;
  history: Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>;
}
