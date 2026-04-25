import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
  setMainWindowAlwaysOnTop,
} from "@/modules/media-player";
import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "@/modules/media-types";

interface UsePlaybackSettingsArgs {
  setHwMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
}

export function usePlaybackSettings({ setHwMode, requestPreviewFrame }: UsePlaybackSettingsArgs) {
  async function applyHwDecodePreference(enabled: boolean) {
    const mode: HardwareDecodeMode = enabled ? "auto" : "off";
    try {
      return await setHwMode(mode);
    } catch {
      // Keep silent here; player surface already emits error events.
      return null;
    }
  }

  async function applyAlwaysOnTopPreference(enabled: boolean) {
    try {
      await setMainWindowAlwaysOnTop(enabled);
    } catch {
      // Keep silent here; user action should not break playback flow.
    }
  }

  function requestTimelinePreview(
    positionSeconds: number,
    maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
    maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  ) {
    return requestPreviewFrame(positionSeconds, maxWidth, maxHeight);
  }

  return {
    applyHwDecodePreference,
    applyAlwaysOnTopPreference,
    requestTimelinePreview,
  };
}
