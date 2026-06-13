import type { UnlistenFn } from "@tauri-apps/api/event";
import type { MediaSnapshot } from "@/modules/media-types";
import { registerSessionEvents } from "./sessionEvents";
import { startSessionTimers } from "./sessionTimers";
import { useMediaSessionState } from "./useMediaSessionState";

export function useMediaSession() {
  const state = useMediaSessionState();
  let sessionUnlisteners: UnlistenFn[] = [];
  let timerDisposer: { dispose: () => void } | null = null;

  async function mount(getSnapshot: () => Promise<MediaSnapshot>) {
    state.updateSnapshot(await getSnapshot());
    sessionUnlisteners = await registerSessionEvents({
      onSnapshot: state.updateSnapshot,
      onMetadata: state.applyMetadataPayload,
      onError: state.applyErrorPayload,
      onTelemetry: state.applyTelemetryPayload,
      onAudioMeter: state.applyAudioMeterPayload,
    });
    timerDisposer = startSessionTimers({
      markTelemetryStaleIfNeeded: state.markTelemetryStaleIfNeeded,
    });
  }

  function unmount() {
    sessionUnlisteners.forEach((unlisten) => unlisten());
    sessionUnlisteners = [];
    timerDisposer?.dispose();
    timerDisposer = null;
  }

  return {
    snapshot: state.snapshot,
    initialized: state.initialized,
    currentSource: state.currentSource,
    latestAudioMeter: state.latestAudioMeter,
    networkReadBytesPerSecond: state.networkReadBytesPerSecond,
    networkSustainRatio: state.networkSustainRatio,
    metadataDurationSeconds: state.metadataDurationSeconds,
    metadataMediaKind: state.metadataMediaKind,
    metadataTitle: state.metadataTitle,
    metadataArtist: state.metadataArtist,
    metadataAlbum: state.metadataAlbum,
    metadataHasCoverArt: state.metadataHasCoverArt,
    metadataLyrics: state.metadataLyrics,
    metadataLyricsSource: state.metadataLyricsSource,
    metadataLyricsCandidateId: state.metadataLyricsCandidateId,
    metadataLyricsCandidates: state.metadataLyricsCandidates,
    metadataLyricsFetching: state.metadataLyricsFetching,
    metadataVideoWidth: state.metadataVideoWidth,
    metadataVideoHeight: state.metadataVideoHeight,
    metadataVideoFps: state.metadataVideoFps,
    playbackErrorMessage: state.playbackErrorMessage,
    mount,
    unmount,
    updateSnapshot: state.updateSnapshot,
  };
}
