import { ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  MEDIA_DEBUG_EVENT,
  MEDIA_DEBUG_EVENT_V2,
  MEDIA_ERROR_EVENT,
  MEDIA_METADATA_EVENT,
  MEDIA_MENU_EVENT,
  MEDIA_STATE_EVENT,
  MEDIA_STATE_EVENT_V2,
  type MediaEventEnvelope,
  type MediaDebugPayload,
  type MediaErrorPayload,
  type MediaMetadataPayload,
  type MediaSnapshot,
  MEDIA_TELEMETRY_EVENT_V2,
  type MediaTelemetryPayload,
} from "@/modules/media-types";

export function useMediaSession() {
  const snapshot = ref<MediaSnapshot | null>(null);
  const currentSource = ref("");
  const debugSnapshot = ref<Record<string, string>>({});
  const metadataDurationSeconds = ref<number | null>(null);
  const playbackErrorMessage = ref("");
  let unlistenMediaEvent: UnlistenFn | null = null;
  let unlistenMediaEventV2: UnlistenFn | null = null;
  let unlistenMenuEvent: UnlistenFn | null = null;
  let unlistenMetadataEvent: UnlistenFn | null = null;
  let unlistenErrorEvent: UnlistenFn | null = null;
  let unlistenDebugEvent: UnlistenFn | null = null;
  let unlistenDebugEventV2: UnlistenFn | null = null;
  let unlistenTelemetryEventV2: UnlistenFn | null = null;
  let snapshotPollingTimer: number | null = null;

  function updateSnapshot(next: MediaSnapshot) {
    snapshot.value = next;
    currentSource.value = next.playback.current_path ?? "";
  }

  async function mount(
    onMenuAction: (action: string) => void,
    getSnapshot: () => Promise<MediaSnapshot>,
  ) {
    updateSnapshot(await getSnapshot());
    unlistenMediaEvent = await listen<MediaSnapshot>(MEDIA_STATE_EVENT, (event) => {
      updateSnapshot(event.payload);
    });
    unlistenMediaEventV2 = await listen<MediaEventEnvelope<MediaSnapshot>>(MEDIA_STATE_EVENT_V2, (event) => {
      updateSnapshot(event.payload.payload);
    });
    unlistenMenuEvent = await listen<string>(MEDIA_MENU_EVENT, (event) => {
      onMenuAction(event.payload);
    });
    unlistenMetadataEvent = await listen<MediaMetadataPayload>(MEDIA_METADATA_EVENT, (event) => {
      metadataDurationSeconds.value = event.payload.duration_seconds;
    });
    unlistenErrorEvent = await listen<MediaErrorPayload>(MEDIA_ERROR_EVENT, (event) => {
      playbackErrorMessage.value = `${event.payload.code}: ${event.payload.message}`;
    });
    const upsertDebug = (payload: MediaDebugPayload) => {
      const stage = payload.stage?.trim() || "debug";
      const msg = payload.message?.trim() || "";
      debugSnapshot.value = {
        ...debugSnapshot.value,
        [stage]: msg || "-",
      };
    };
    unlistenDebugEvent = await listen<MediaDebugPayload>(MEDIA_DEBUG_EVENT, (event) => {
      upsertDebug(event.payload);
    });
    unlistenDebugEventV2 = await listen<MediaEventEnvelope<MediaDebugPayload>>(MEDIA_DEBUG_EVENT_V2, (event) => {
      upsertDebug(event.payload.payload);
    });
    unlistenTelemetryEventV2 = await listen<MediaEventEnvelope<MediaTelemetryPayload>>(
      MEDIA_TELEMETRY_EVENT_V2,
      (event) => {
        const p = event.payload.payload;
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
    unlistenMediaEvent?.();
    unlistenMediaEvent = null;
    unlistenMediaEventV2?.();
    unlistenMediaEventV2 = null;
    unlistenMenuEvent?.();
    unlistenMenuEvent = null;
    unlistenMetadataEvent?.();
    unlistenMetadataEvent = null;
    unlistenErrorEvent?.();
    unlistenErrorEvent = null;
    unlistenDebugEvent?.();
    unlistenDebugEvent = null;
    unlistenDebugEventV2?.();
    unlistenDebugEventV2 = null;
    unlistenTelemetryEventV2?.();
    unlistenTelemetryEventV2 = null;
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
