import type { Ref } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaErrorPayload,
  MediaMetadataPayload,
  MediaSnapshot,
  MediaTelemetryPayload,
  PlaybackState,
} from "@/modules/media-types";

export interface MediaSessionStateRefs {
  currentSource: Ref<string>;
  networkReadBytesPerSecond: Ref<number | null>;
  networkSustainRatio: Ref<number | null>;
  lastTelemetryAtMs: Ref<number>;
  telemetryStaleTimeoutId: Ref<number | null>;
}

export interface MediaSessionStateHandlers {
  applyAudioMeterPayload: (payload: MediaAudioMeterPayload) => void;
  applyErrorPayload: (payload: MediaErrorPayload) => void;
  applyMetadataPayload: (payload: MediaMetadataPayload) => void;
  applyPlaybackProgressPayload: (payload: PlaybackState) => void;
  applyTelemetryPayload: (payload: MediaTelemetryPayload) => void;
  disposeTelemetryStaleTimeout: () => void;
  resetTransientMediaState: () => void;
  updateSnapshot: (next: MediaSnapshot) => void;
}

export type TelemetryPayloadHandler = (payload: MediaTelemetryPayload) => void;
export type AudioMeterPayloadHandler = (payload: MediaAudioMeterPayload) => void;
export type SnapshotUpdater = (next: MediaSnapshot) => void;
