import {
  DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
} from "@/modules/media-player";
import {
  applyAlwaysOnTopPreference,
  applyHwDecodePreference,
  applyVideoScaleModePreference,
} from "@/modules/player-settings-actions";
import type { PlayerVideoScaleMode } from "@/modules/preferences";
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
  async function applyHwDecode(enabled: boolean) {
    return applyHwDecodePreference(enabled, setHwMode);
  }

  async function applyAlwaysOnTop(enabled: boolean) {
    await applyAlwaysOnTopPreference(enabled);
  }

  async function applyVideoScaleMode(mode: PlayerVideoScaleMode) {
    await applyVideoScaleModePreference(mode);
  }

  function requestTimelinePreview(
    positionSeconds: number,
    maxWidth = DEFAULT_PREVIEW_FRAME_MAX_WIDTH,
    maxHeight = DEFAULT_PREVIEW_FRAME_MAX_HEIGHT,
  ) {
    return requestPreviewFrame(positionSeconds, maxWidth, maxHeight);
  }

  return {
    applyHwDecode,
    applyAlwaysOnTop,
    applyVideoScaleMode,
    requestTimelinePreview,
  };
}
