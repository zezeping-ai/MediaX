import type { UnlistenFn } from "@tauri-apps/api/event";
import type { MediaSnapshot } from "@/modules/media-types";
import { registerSessionEvents } from "./sessionEvents";
import { useMediaSessionState } from "./useMediaSessionState";

export function useMediaSession() {
  const state = useMediaSessionState();
  let sessionUnlisteners: UnlistenFn[] = [];

  async function mount(
    onMenuAction: (action: string) => void,
    getSnapshot: () => Promise<MediaSnapshot>,
  ) {
    state.updateSnapshot(await getSnapshot());
    sessionUnlisteners = await registerSessionEvents({
      onMenuAction,
      onPlaybackProgress: state.applyPlaybackProgressPayload,
      onSnapshot: state.updateSnapshot,
      onMetadata: state.applyMetadataPayload,
      onError: state.applyErrorPayload,
      onTelemetry: state.applyTelemetryPayload,
      onAudioMeter: state.applyAudioMeterPayload,
    });
  }

  function unmount() {
    sessionUnlisteners.forEach((unlisten) => unlisten());
    sessionUnlisteners = [];
    state.disposeTelemetryStaleTimeout();
  }

  return {
    mount,
    unmount,
    ...state,
  };
}
