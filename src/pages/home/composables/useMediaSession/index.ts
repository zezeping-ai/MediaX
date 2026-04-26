import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_PLAYBACK_DEBUG_EVENT,
  MEDIA_PLAYBACK_ERROR_EVENT,
  MEDIA_PLAYBACK_METADATA_EVENT,
  MEDIA_PLAYBACK_STATE_EVENT,
  MEDIA_PLAYBACK_TELEMETRY_EVENT,
  MEDIA_MENU_EVENT,
  type MediaDebugPayload,
  type MediaErrorPayload,
  type MediaEventEnvelope,
  type MediaMetadataPayload,
  type MediaSnapshot,
  type MediaTelemetryPayload,
} from "@/modules/media-types";
import { useMediaSessionState } from "./useMediaSessionState";

export function useMediaSession() {
  const state = useMediaSessionState();
  let unlistenPlaybackStateEvent: UnlistenFn | null = null;
  let unlistenMenuEvent: UnlistenFn | null = null;
  let unlistenPlaybackMetadataEvent: UnlistenFn | null = null;
  let unlistenPlaybackErrorEvent: UnlistenFn | null = null;
  let unlistenPlaybackDebugEvent: UnlistenFn | null = null;
  let unlistenPlaybackTelemetryEvent: UnlistenFn | null = null;
  let snapshotPollingTimer: number | null = null;
  let telemetryStaleTimer: number | null = null;

  async function mount(
    onMenuAction: (action: string) => void,
    getSnapshot: () => Promise<MediaSnapshot>,
  ) {
    state.updateSnapshot(await getSnapshot());
    unlistenPlaybackStateEvent = await listen<MediaEventEnvelope<MediaSnapshot> | MediaSnapshot>(
      MEDIA_PLAYBACK_STATE_EVENT,
      (event) => {
        state.updateSnapshot(resolvePayload(event.payload));
      },
    );
    unlistenMenuEvent = await listen<string>(MEDIA_MENU_EVENT, (event) => {
      onMenuAction(event.payload);
    });
    unlistenPlaybackMetadataEvent = await listen<
      MediaEventEnvelope<MediaMetadataPayload> | MediaMetadataPayload
    >(
      MEDIA_PLAYBACK_METADATA_EVENT,
      (event) => {
        state.applyMetadataPayload(resolvePayload(event.payload));
      },
    );
    unlistenPlaybackErrorEvent = await listen<
      MediaEventEnvelope<MediaErrorPayload> | MediaErrorPayload
    >(
      MEDIA_PLAYBACK_ERROR_EVENT,
      (event) => {
        state.applyErrorPayload(resolvePayload(event.payload));
      },
    );
    unlistenPlaybackDebugEvent = await listen<
      MediaEventEnvelope<MediaDebugPayload> | MediaDebugPayload
    >(
      MEDIA_PLAYBACK_DEBUG_EVENT,
      (event) => {
        state.applyDebugPayload(resolvePayload(event.payload));
      },
    );
    unlistenPlaybackTelemetryEvent = await listen<
      MediaEventEnvelope<MediaTelemetryPayload> | MediaTelemetryPayload
    >(
      MEDIA_PLAYBACK_TELEMETRY_EVENT,
      (event) => {
        state.applyTelemetryPayload(resolvePayload(event.payload));
      },
    );
    snapshotPollingTimer = window.setInterval(() => {
      void getSnapshot().then(state.updateSnapshot);
    }, 1000);
    telemetryStaleTimer = window.setInterval(() => {
      state.markTelemetryStaleIfNeeded();
    }, 500);
  }

  function unmount() {
    unlistenPlaybackStateEvent?.();
    unlistenPlaybackStateEvent = null;
    unlistenMenuEvent?.();
    unlistenMenuEvent = null;
    unlistenPlaybackMetadataEvent?.();
    unlistenPlaybackMetadataEvent = null;
    unlistenPlaybackErrorEvent?.();
    unlistenPlaybackErrorEvent = null;
    unlistenPlaybackDebugEvent?.();
    unlistenPlaybackDebugEvent = null;
    unlistenPlaybackTelemetryEvent?.();
    unlistenPlaybackTelemetryEvent = null;
    if (snapshotPollingTimer !== null) {
      window.clearInterval(snapshotPollingTimer);
      snapshotPollingTimer = null;
    }
    if (telemetryStaleTimer !== null) {
      window.clearInterval(telemetryStaleTimer);
      telemetryStaleTimer = null;
    }
  }

  return {
    snapshot: state.snapshot,
    currentSource: state.currentSource,
    debugSnapshot: state.debugSnapshot,
    debugTimeline: state.debugTimeline,
    debugStageSnapshot: state.debugStageSnapshot,
    firstFrameAtMs: state.firstFrameAtMs,
    latestTelemetry: state.latestTelemetry,
    telemetryHistory: state.telemetryHistory,
    networkReadBytesPerSecond: state.networkReadBytesPerSecond,
    networkSustainRatio: state.networkSustainRatio,
    metadataDurationSeconds: state.metadataDurationSeconds,
    metadataVideoWidth: state.metadataVideoWidth,
    metadataVideoHeight: state.metadataVideoHeight,
    metadataVideoFps: state.metadataVideoFps,
    playbackErrorMessage: state.playbackErrorMessage,
    mount,
    unmount,
    updateSnapshot: state.updateSnapshot,
  };
}

function resolvePayload<T>(payload: T | MediaEventEnvelope<T>): T {
  if (payload && typeof payload === "object" && "payload" in payload) {
    return (payload as MediaEventEnvelope<T>).payload;
  }
  return payload as T;
}
