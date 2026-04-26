import type { Ref } from "vue";
import type {
  MediaDebugPayload,
  MediaSnapshot,
  MediaTelemetryPayload,
} from "@/modules/media-types";

export interface MediaSessionStateRefs {
  currentSource: Ref<string>;
  debugSnapshot: Ref<Record<string, string>>;
  debugTimeline: Ref<Array<{ stage: string; message: string; at_ms: number }>>;
  debugStageSnapshot: Ref<Record<string, { message: string; at_ms: number }>>;
  latestTelemetry: Ref<MediaTelemetryPayload | null>;
  telemetryHistory: Ref<Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>>;
  firstFrameAtMs: Ref<number | null>;
  networkReadBytesPerSecond: Ref<number | null>;
  networkSustainRatio: Ref<number | null>;
  lastTelemetryAtMs: Ref<number>;
}

export type DebugPayloadHandler = (payload: MediaDebugPayload) => void;
export type TelemetryPayloadHandler = (payload: MediaTelemetryPayload) => void;
export type SnapshotUpdater = (next: MediaSnapshot) => void;
