import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_PLAYBACK_AUDIO_METER_EVENT,
  MEDIA_MENU_EVENT,
  MEDIA_PLAYBACK_ERROR_EVENT,
  MEDIA_PLAYBACK_METADATA_EVENT,
  MEDIA_PLAYBACK_STATE_EVENT,
  MEDIA_PLAYBACK_TELEMETRY_EVENT,
  type MediaAudioMeterPayload,
  type MediaErrorPayload,
  type MediaEventEnvelope,
  type MediaMetadataPayload,
  type MediaSnapshot,
  type MediaTelemetryPayload,
} from "@/modules/media-types";

interface SessionEventBindings {
  onMenuAction: (action: string) => void;
  onSnapshot: (snapshot: MediaSnapshot) => void;
  onMetadata: (payload: MediaMetadataPayload) => void;
  onError: (payload: MediaErrorPayload) => void;
  onTelemetry: (payload: MediaTelemetryPayload) => void;
  onAudioMeter: (payload: MediaAudioMeterPayload) => void;
}

export async function registerSessionEvents(bindings: SessionEventBindings) {
  const unlistenPlaybackStateEvent = await listen<
    MediaEventEnvelope<MediaSnapshot> | MediaSnapshot
  >(MEDIA_PLAYBACK_STATE_EVENT, (event) => {
    bindings.onSnapshot(resolvePayload(event.payload));
  });
  const unlistenMenuEvent = await listen<string>(MEDIA_MENU_EVENT, (event) => {
    bindings.onMenuAction(event.payload);
  });
  const unlistenPlaybackMetadataEvent = await listen<
    MediaEventEnvelope<MediaMetadataPayload> | MediaMetadataPayload
  >(MEDIA_PLAYBACK_METADATA_EVENT, (event) => {
    bindings.onMetadata(resolvePayload(event.payload));
  });
  const unlistenPlaybackErrorEvent = await listen<
    MediaEventEnvelope<MediaErrorPayload> | MediaErrorPayload
  >(MEDIA_PLAYBACK_ERROR_EVENT, (event) => {
    bindings.onError(resolvePayload(event.payload));
  });
  const unlistenPlaybackTelemetryEvent = await listen<
    MediaEventEnvelope<MediaTelemetryPayload> | MediaTelemetryPayload
  >(MEDIA_PLAYBACK_TELEMETRY_EVENT, (event) => {
    bindings.onTelemetry(resolvePayload(event.payload));
  });
  const unlistenPlaybackAudioMeterEvent = await listen<
    MediaEventEnvelope<MediaAudioMeterPayload> | MediaAudioMeterPayload
  >(MEDIA_PLAYBACK_AUDIO_METER_EVENT, (event) => {
    bindings.onAudioMeter(resolvePayload(event.payload));
  });

  return [
    unlistenPlaybackStateEvent,
    unlistenMenuEvent,
    unlistenPlaybackMetadataEvent,
    unlistenPlaybackErrorEvent,
    unlistenPlaybackTelemetryEvent,
    unlistenPlaybackAudioMeterEvent,
  ] satisfies UnlistenFn[];
}

function resolvePayload<T>(payload: T | MediaEventEnvelope<T>): T {
  if (payload && typeof payload === "object" && "payload" in payload) {
    return (payload as MediaEventEnvelope<T>).payload;
  }
  return payload as T;
}
