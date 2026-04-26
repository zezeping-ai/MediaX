import { ref, shallowRef } from "vue";
import type {
  MediaAudioMeterPayload,
  MediaErrorPayload,
  MediaLyricLine,
  MediaMetadataPayload,
  PlaybackMediaKind,
  MediaSnapshot,
  MediaTelemetryPayload,
} from "@/modules/media-types";
import { toUserMediaErrorMessage } from "../../useMediaErrorMap";
import { createDebugPayloadHandler } from "./createDebugPayloadHandler";
import { createTelemetryPayloadHandler } from "./createTelemetryPayloadHandler";

export function useMediaSessionState() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const debugSnapshot = ref<Record<string, string>>({});
  const debugTimeline = ref<Array<{ stage: string; message: string; at_ms: number }>>([]);
  const debugStageSnapshot = ref<Record<string, { message: string; at_ms: number }>>({});
  const latestTelemetry = shallowRef<MediaTelemetryPayload | null>(null);
  const latestAudioMeter = shallowRef<MediaAudioMeterPayload | null>(null);
  const telemetryHistory = ref<Array<{ at_ms: number; telemetry: MediaTelemetryPayload }>>([]);
  const firstFrameAtMs = ref<number | null>(null);
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

  function resetTransientMediaState() {
    debugSnapshot.value = {};
    debugTimeline.value = [];
    debugStageSnapshot.value = {};
    firstFrameAtMs.value = null;
    latestTelemetry.value = null;
    latestAudioMeter.value = null;
    telemetryHistory.value = [];
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
    playbackErrorMessage.value = "";
  }

  function updateSnapshot(next: MediaSnapshot) {
    const previousSource = currentSource.value;
    snapshot.value = next;
    currentSource.value = next.playback.current_path ?? "";
    if (previousSource !== currentSource.value) {
      resetTransientMediaState();
    }
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

  const applyDebugPayload = createDebugPayloadHandler({
    currentSource,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    latestTelemetry,
    latestAudioMeter,
    telemetryHistory,
    firstFrameAtMs,
    networkReadBytesPerSecond,
    networkSustainRatio,
    lastTelemetryAtMs,
  });

  const applyTelemetryPayload = createTelemetryPayloadHandler({
    currentSource,
    debugSnapshot,
    debugTimeline,
    debugStageSnapshot,
    latestTelemetry,
    latestAudioMeter,
    telemetryHistory,
    firstFrameAtMs,
    networkReadBytesPerSecond,
    networkSustainRatio,
    lastTelemetryAtMs,
  });

  function applyAudioMeterPayload(payload: MediaAudioMeterPayload) {
    latestAudioMeter.value = payload;
  }

  function markTelemetryStaleIfNeeded() {
    if (!currentSource.value) {
      return;
    }
    if (!lastTelemetryAtMs.value) {
      return;
    }
    if (Date.now() - lastTelemetryAtMs.value >= 2000) {
      networkReadBytesPerSecond.value = 0;
      networkSustainRatio.value = null;
    }
  }

  return {
    applyDebugPayload,
    applyAudioMeterPayload,
    applyErrorPayload,
    applyMetadataPayload,
    applyTelemetryPayload,
    currentSource,
    debugSnapshot,
    debugStageSnapshot,
    debugTimeline,
    firstFrameAtMs,
    latestTelemetry,
    latestAudioMeter,
    markTelemetryStaleIfNeeded,
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
    telemetryHistory,
    updateSnapshot,
  };
}
