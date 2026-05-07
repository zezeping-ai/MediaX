import { onBeforeUnmount, watch, type Ref } from "vue";
import { setMainWindowTitle } from "@/modules/media-player/windowCommands";
import type { createPlaybackCommandRunner } from "./createPlaybackCommandRunner";
import type { useMediaSession } from "../useMediaSession";

type MediaSession = ReturnType<typeof useMediaSession>;
type PlaybackRunner = ReturnType<typeof createPlaybackCommandRunner>;

type SyncMediaCenterRuntimeOptions = {
  mediaSession: Pick<MediaSession, "currentSource" | "metadataDurationSeconds" | "playbackErrorMessage" | "snapshot" | "unmount">;
  playbackRunner: Pick<PlaybackRunner, "syncPosition">;
  errorMessage: Ref<string>;
};

export function syncMediaCenterRuntime(options: SyncMediaCenterRuntimeOptions) {
  onBeforeUnmount(() => {
    void setMainWindowTitle(null);
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

  watch(
    () => ({
      source: options.mediaSession.currentSource.value,
      status: options.mediaSession.snapshot.value?.playback.status ?? "idle",
    }),
    ({ source, status }) => {
      const shouldUseSourceTitle = Boolean(source) && status !== "idle" && status !== "stopped";
      void setMainWindowTitle(shouldUseSourceTitle ? decodeTitleSource(source) : null);
    },
    { immediate: true },
  );
}

function decodeTitleSource(source: string) {
  const trimmed = source.trim();
  if (!trimmed.includes("%")) {
    return trimmed;
  }
  const uriDecoded = tryDecode(trimmed, decodeURI);
  if (!uriDecoded.includes("%")) {
    return uriDecoded;
  }
  return tryDecode(uriDecoded, decodeURIComponent);
}

function tryDecode(value: string, decoder: (input: string) => string) {
  try {
    return decoder(value);
  } catch {
    return value;
  }
}
