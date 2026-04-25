import { ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_PLAYBACK_DEBUG_EVENT,
  MEDIA_PLAYBACK_ERROR_EVENT,
  MEDIA_PLAYBACK_METADATA_EVENT,
  MEDIA_PLAYBACK_STATE_EVENT,
  MEDIA_PLAYBACK_TELEMETRY_EVENT,
  MEDIA_MENU_EVENT,
  type MediaEventEnvelope,
  type MediaDebugPayload,
  type MediaErrorPayload,
  type MediaMetadataPayload,
  type MediaSnapshot,
  type MediaTelemetryPayload,
} from "@/modules/media-types";

export function useMediaSession() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const debugSnapshot = ref<Record<string, string>>({});
  const metadataDurationSeconds = ref<number | null>(null);
  const playbackErrorMessage = ref("");
  let unlistenPlaybackStateEvent: UnlistenFn | null = null;
  let unlistenMenuEvent: UnlistenFn | null = null;
  let unlistenPlaybackMetadataEvent: UnlistenFn | null = null;
  let unlistenPlaybackErrorEvent: UnlistenFn | null = null;
  let unlistenPlaybackDebugEvent: UnlistenFn | null = null;
  let unlistenPlaybackTelemetryEvent: UnlistenFn | null = null;
  let snapshotPollingTimer: number | null = null;

  function updateSnapshot(next: MediaSnapshot) {
    snapshot.value = next;
    currentSource.value = next.playback.current_path ?? "";
  }

  function resolvePayload<T>(payload: T | MediaEventEnvelope<T>): T {
    if (payload && typeof payload === "object" && "payload" in payload) {
      return (payload as MediaEventEnvelope<T>).payload;
    }
    return payload as T;
  }

  async function mount(
    onMenuAction: (action: string) => void,
    getSnapshot: () => Promise<MediaSnapshot>,
  ) {
    updateSnapshot(await getSnapshot());
    unlistenPlaybackStateEvent = await listen<MediaEventEnvelope<MediaSnapshot> | MediaSnapshot>(
      MEDIA_PLAYBACK_STATE_EVENT,
      (event) => {
        updateSnapshot(resolvePayload(event.payload));
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
        metadataDurationSeconds.value = resolvePayload(event.payload).duration_seconds;
      },
    );
    unlistenPlaybackErrorEvent = await listen<MediaEventEnvelope<MediaErrorPayload> | MediaErrorPayload>(
      MEDIA_PLAYBACK_ERROR_EVENT,
      (event) => {
        const payload = resolvePayload(event.payload);
        playbackErrorMessage.value = `${payload.code}: ${payload.message}`;
      },
    );
    const upsertDebug = (payload: MediaDebugPayload) => {
      const stage = payload.stage?.trim() || "debug";
      const msg = payload.message?.trim() || "";
      debugSnapshot.value = {
        ...debugSnapshot.value,
        [stage]: msg || "-",
      };
    };
    unlistenPlaybackDebugEvent = await listen<MediaEventEnvelope<MediaDebugPayload> | MediaDebugPayload>(
      MEDIA_PLAYBACK_DEBUG_EVENT,
      (event) => {
        upsertDebug(resolvePayload(event.payload));
      },
    );
    unlistenPlaybackTelemetryEvent = await listen<
      MediaEventEnvelope<MediaTelemetryPayload> | MediaTelemetryPayload
    >(
      MEDIA_PLAYBACK_TELEMETRY_EVENT,
      (event) => {
        const p = resolvePayload(event.payload);
        debugSnapshot.value = {
          ...debugSnapshot.value,
          telemetry: `src=${p.source_fps.toFixed(2)} render=${p.render_fps.toFixed(2)} queue=${p.queue_depth} drift=${(p.audio_drift_seconds ?? 0).toFixed(3)}s`,
        };
      },
    );
    snapshotPollingTimer = window.setInterval(() => {
      void getSnapshot().then(updateSnapshot);
    }, 1000);
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
  }

  return {
    snapshot,
    currentSource,
    debugSnapshot,
    metadataDurationSeconds,
    playbackErrorMessage,
    mount,
    unmount,
    updateSnapshot,
  };
}
