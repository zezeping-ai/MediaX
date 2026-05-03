import { ref, shallowRef } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaErrorPayload,
  MediaLyricLine,
  MediaMetadataPayload,
  PlaybackMediaKind,
  PlaybackState,
  MediaSnapshot,
} from "@/modules/media-types";
import { toUserMediaErrorMessage } from "../../useMediaErrorMap";
import { createTelemetryPayloadHandler } from "./createTelemetryPayloadHandler";
import type { MediaSessionStateHandlers } from "./types";

export function useMediaSessionState() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const latestAudioMeter = shallowRef<MediaAudioMeterPayload | null>(null);
  const metadataDurationSeconds = ref<number | null>(null);
  const metadataMediaKind = ref<PlaybackMediaKind>("video");
  const metadataVideoWidth = ref<number | null>(null);
  const metadataVideoHeight = ref<number | null>(null);
  const metadataVideoFps = ref<number | null>(null);
  const metadataTitle = ref("");
  const metadataArtist = ref("");
  const metadataAlbum = ref("");
  const metadataHasCoverArt = ref(false);
  const metadataLyrics = ref<MediaLyricLine[]>([]);
  const playbackErrorMessage = ref("");
  const networkReadBytesPerSecond = ref<number | null>(null);
  const networkSustainRatio = ref<number | null>(null);
  const lastTelemetryAtMs = ref(0);
  const telemetryStaleTimeoutId = ref<number | null>(null);

  function resetTransientMediaState() {
    latestAudioMeter.value = null;
    metadataDurationSeconds.value = null;
    metadataMediaKind.value = "video";
    metadataVideoWidth.value = null;
    metadataVideoHeight.value = null;
    metadataVideoFps.value = null;
    metadataTitle.value = "";
    metadataArtist.value = "";
    metadataAlbum.value = "";
    metadataHasCoverArt.value = false;
    metadataLyrics.value = [];
    networkReadBytesPerSecond.value = null;
    networkSustainRatio.value = null;
    lastTelemetryAtMs.value = 0;
    if (telemetryStaleTimeoutId.value !== null) {
      window.clearTimeout(telemetryStaleTimeoutId.value);
      telemetryStaleTimeoutId.value = null;
    }
    playbackErrorMessage.value = "";
  }

  function handleSourceChanged(nextSource: string) {
    if (currentSource.value === nextSource) {
      return;
    }
    currentSource.value = nextSource;
    resetTransientMediaState();
  }

  function updateSnapshot(next: MediaSnapshot) {
    const previousSource = currentSource.value;
    snapshot.value = next;
    const nextSource = next.playback.current_path ?? "";
    if (previousSource !== nextSource) {
      handleSourceChanged(nextSource);
    }
    metadataMediaKind.value = next.playback.media_kind;
  }

  function applyPlaybackProgressPayload(payload: PlaybackState) {
    const previousSource = currentSource.value;
    const nextSource = payload.current_path ?? "";
    if (previousSource !== nextSource) {
      handleSourceChanged(nextSource);
    }
    snapshot.value = {
      playback: payload,
      library: snapshot.value?.library ?? { roots: [], items: [] },
    };
    metadataMediaKind.value = payload.media_kind;
  }

  function applyMetadataPayload(payload: MediaMetadataPayload) {
    metadataMediaKind.value = payload.media_kind;
    metadataDurationSeconds.value = payload.duration_seconds;
    metadataVideoWidth.value = payload.width;
    metadataVideoHeight.value = payload.height;
    metadataVideoFps.value = payload.fps;
    metadataTitle.value = payload.title ?? "";
    metadataArtist.value = payload.artist ?? "";
    metadataAlbum.value = payload.album ?? "";
    metadataHasCoverArt.value = Boolean(payload.has_cover_art);
    metadataLyrics.value = payload.lyrics ?? [];
  }

  function applyErrorPayload(payload: MediaErrorPayload) {
    playbackErrorMessage.value = toUserMediaErrorMessage(`${payload.code}: ${payload.message}`);
  }

  const applyTelemetryPayload = createTelemetryPayloadHandler({
    currentSource,
    networkReadBytesPerSecond,
    networkSustainRatio,
    lastTelemetryAtMs,
    telemetryStaleTimeoutId,
  });

  function applyAudioMeterPayload(payload: MediaAudioMeterPayload) {
    latestAudioMeter.value = payload;
  }

  function disposeTelemetryStaleTimeout() {
    if (telemetryStaleTimeoutId.value === null) {
      return;
    }
    window.clearTimeout(telemetryStaleTimeoutId.value);
    telemetryStaleTimeoutId.value = null;
  }

  const handlers: MediaSessionStateHandlers = {
    applyAudioMeterPayload,
    applyErrorPayload,
    applyMetadataPayload,
    applyPlaybackProgressPayload,
    applyTelemetryPayload,
    disposeTelemetryStaleTimeout,
    resetTransientMediaState,
    updateSnapshot,
  };

  return {
    ...handlers,
    currentSource,
    latestAudioMeter,
    metadataDurationSeconds,
    metadataMediaKind,
    metadataTitle,
    metadataArtist,
    metadataAlbum,
    metadataHasCoverArt,
    metadataLyrics,
    metadataVideoFps,
    metadataVideoHeight,
    metadataVideoWidth,
    networkReadBytesPerSecond,
    networkSustainRatio,
    playbackErrorMessage,
    snapshot,
  };
}
