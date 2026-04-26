import type { Ref } from "vue";
import { getPlaybackSnapshot } from "@/modules/media-player";

interface UseMediaCenterLifecycleOptions {
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
    openPath: (path: string) => Promise<void>;
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
    await options.withBusyState(refreshSnapshot);
    await options.cacheRecordingController.refreshCacheRecordingStatus();
    if (options.cacheRecordingController.cacheRecording.value) {
      options.cacheRecordingController.startRecordingClock();
      options.cacheRecordingController.startCacheStatusPoll();
    }
    await options.mediaSession.mount((action) => {
      if (action === "open_local") {
        void options.withBusyState(async () => {
          const selectedPath = await options.playbackRunner.openLocalFileByDialog();
          if (selectedPath) {
            await options.playbackRunner.openPath(selectedPath);
          }
        });
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
