import type { Ref } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaSnapshot,
  MediaTelemetryPayload,
} from "@/modules/media-types";

export interface MediaSessionStateRefs {
  currentSource: Ref<string>;
  networkReadBytesPerSecond: Ref<number | null>;
  networkSustainRatio: Ref<number | null>;
  lastTelemetryAtMs: Ref<number>;
}

export type TelemetryPayloadHandler = (payload: MediaTelemetryPayload) => void;
export type AudioMeterPayloadHandler = (payload: MediaAudioMeterPayload) => void;
export type SnapshotUpdater = (next: MediaSnapshot) => void;
