import type { HardwareDecodeMode, MediaSnapshot, PreviewFrame } from "@/modules/media-types";
import { createPlaybackPreferenceAppliers } from "./applyPlaybackPreferences";
import { createTimelinePreviewRequester } from "./createTimelinePreviewRequester";

interface UsePlaybackSettingsArgs {
  configureDecoderMode: (mode: HardwareDecodeMode) => Promise<MediaSnapshot>;
  requestPreviewFrame: (
    positionSeconds: number,
    maxWidth?: number,
    maxHeight?: number,
  ) => Promise<PreviewFrame | null>;
}

export function usePlaybackSettings({ configureDecoderMode, requestPreviewFrame }: UsePlaybackSettingsArgs) {
  const playbackPreferenceAppliers = createPlaybackPreferenceAppliers(configureDecoderMode);
  const timelinePreviewRequester = createTimelinePreviewRequester(requestPreviewFrame);

  return {
    ...playbackPreferenceAppliers,
    ...timelinePreviewRequester,
  };
}
