import type { Ref } from "vue";
import { getPlaybackSnapshot } from "@/modules/media-player";

interface UseMediaCenterLifecycleOptions {
  captureError: (error: unknown) => void;
  withBusyState: (action: () => Promise<void>) => Promise<void>;
  mediaSession: {
    updateSnapshot: (snapshot: Awaited<ReturnType<typeof getPlaybackSnapshot>>) => void;
    mount: (
      onMenuAction: (action: string) => void,
      getSnapshot: () => Promise<Awaited<ReturnType<typeof getPlaybackSnapshot>>>,
    ) => Promise<void>;
  };
  cacheRecordingController: {
    refreshCacheRecordingStatus: () => Promise<void>;
    cacheRecording: Ref<boolean>;
    startRecordingClock: () => void;
    startCacheStatusPoll: () => void;
  };
  playbackRunner: {
    openLocalFileByDialog: () => Promise<string | null>;
    openSource: (source: string) => Promise<void>;
  };
  urlInputController: {
    requestOpenUrlInput: () => void;
  };
}

export function useMediaCenterLifecycle(options: UseMediaCenterLifecycleOptions) {
  async function refreshSnapshot() {
    options.mediaSession.updateSnapshot(await getPlaybackSnapshot());
  }

  async function mountMediaCenter() {
    // Avoid startup loading overlay: blocks idle chrome paint on transparent webviews until IPC returns.
    try {
      await refreshSnapshot();
    } catch (error) {
      options.captureError(error);
    }
    await options.cacheRecordingController.refreshCacheRecordingStatus();
    if (options.cacheRecordingController.cacheRecording.value) {
      options.cacheRecordingController.startRecordingClock();
      options.cacheRecordingController.startCacheStatusPoll();
    }
    await options.mediaSession.mount((action) => {
      if (action === "open_local") {
        void (async () => {
          const selectedPath = await options.playbackRunner.openLocalFileByDialog();
          if (!selectedPath) {
            return;
          }
          await options.withBusyState(async () => {
            await options.playbackRunner.openSource(selectedPath);
          });
        })();
      }
      if (action === "open_url") {
        options.urlInputController.requestOpenUrlInput();
      }
    }, getPlaybackSnapshot);
  }

  return {
    mountMediaCenter,
    refreshSnapshot,
  };
}
