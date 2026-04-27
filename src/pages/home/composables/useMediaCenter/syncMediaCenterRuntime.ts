import { onBeforeUnmount, watch, type Ref } from "vue";
import type { createPlaybackCommandRunner } from "./createPlaybackCommandRunner";
import type { useMediaSession } from "../useMediaSession";

type MediaSession = ReturnType<typeof useMediaSession>;
type PlaybackRunner = ReturnType<typeof createPlaybackCommandRunner>;

type SyncMediaCenterRuntimeOptions = {
  mediaSession: Pick<MediaSession, "metadataDurationSeconds" | "playbackErrorMessage" | "unmount">;
  playbackRunner: Pick<PlaybackRunner, "syncPosition">;
  errorMessage: Ref<string>;
};

export function syncMediaCenterRuntime(options: SyncMediaCenterRuntimeOptions) {
  onBeforeUnmount(() => {
    options.mediaSession.unmount();
  });

  watch(options.mediaSession.metadataDurationSeconds, (duration) => {
    if (typeof duration === "number" && Number.isFinite(duration) && duration > 0) {
      void options.playbackRunner.syncPosition(0, duration);
    }
  });

  watch(options.mediaSession.playbackErrorMessage, (message) => {
    if (message) {
      options.errorMessage.value = message;
    }
  });
}
